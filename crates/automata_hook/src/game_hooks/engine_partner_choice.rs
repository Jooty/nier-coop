use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{Mutex, OnceLock};

use automata_game::address::ENGINE_PARTNER_SPAWN_FUNCTION;
use automata_game::character::Character;
use automata_game::minute_line_budget::MinuteLineBudget;
use automata_game::partner::{
    BODY_MODEL_ID_OFFSET, BODY_PARTNER_SLOT_HANDLE_OFFSET, BUDDY_AUTO_FLAG_HASH, BuddyFlagWrite,
    BuddyFlags, ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN, EnginePartnerChoice, EnginePartnerReadings,
    buddy_flag_hash, choose_engine_partner, engine_partner_spawn_code_is_the_noted_one,
};
use automata_game::start::buddy_flag;
use nt_hook::diagnostics::log_line;

use crate::budgeted_log::log_within;
use crate::game_calls::GameCalls;

const NO_PARTNER_WANTED: u32 = 0;
const CHOICE_LINES_LOGGED_PER_MINUTE: u32 = 60;

static ORIGINAL_ENGINE_PARTNER_SPAWN: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static CHOICE_CALLS: OnceLock<GameCalls> = OnceLock::new();
static WANTED_OBJECT_ID: AtomicU32 = AtomicU32::new(NO_PARTNER_WANTED);
static CHOICE_LINES: Mutex<MinuteLineBudget> =
    Mutex::new(MinuteLineBudget::new(CHOICE_LINES_LOGGED_PER_MINUTE));
static LAST_CHOICE_LINE: Mutex<String> = Mutex::new(String::new());

type EnginePartnerSpawn = unsafe extern "system" fn(usize);

/// Hooks the engine's own partner spawn so it can be made to pick the other player's character.
/// True when the code of this exe is the noted one and the hook is in place.
pub fn install_engine_partner_choice(calls: GameCalls) -> bool {
    let code_is_the_noted_one = calls
        .read_bytes(
            ENGINE_PARTNER_SPAWN_FUNCTION,
            ENGINE_PARTNER_SPAWN_CODE_CHECKED_LEN,
        )
        .is_ok_and(|code| engine_partner_spawn_code_is_the_noted_one(&code));
    if !code_is_the_noted_one {
        log_line(
            "partner: the engine's own partner spawn is not the noted one on this exe; the engine picks its partner by itself and a wrong one is replaced later",
        );
        return false;
    }
    if CHOICE_CALLS.set(calls).is_err() {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "hooks the engine's own partner spawn; the replacement has the signature of the original"
    )]
    // SAFETY: the address is the partner spawn of the verified build and its code was just
    // checked; it takes the hero and gives nothing back, like the replacement.
    unsafe {
        nt_hook::detour::hook_address(
            "engine partner spawn (it is made to pick the other player's character)",
            calls.address_value(ENGINE_PARTNER_SPAWN_FUNCTION) as usize,
            engine_partner_spawn_detour as *const () as *mut c_void,
            &ORIGINAL_ENGINE_PARTNER_SPAWN,
        )
    }
}

/// The character the engine's own partner spawn is to make from now on; `None` leaves it alone.
pub fn set_partner_the_engine_should_make(wanted: Option<Character>) {
    WANTED_OBJECT_ID.store(
        wanted.map_or(NO_PARTNER_WANTED, Character::object_id),
        Ordering::Release,
    );
}

fn slot_is_empty(calls: &GameCalls, hero: usize) -> bool {
    calls
        .read_u32_at(calls.body_field(hero, BODY_PARTNER_SLOT_HANDLE_OFFSET))
        .is_ok_and(|handle| calls.entity_of_handle(handle) == 0)
}

fn character_of(calls: &GameCalls, body: usize) -> Option<Character> {
    calls
        .read_u32_at(calls.body_field(body, BODY_MODEL_ID_OFFSET))
        .ok()
        .and_then(Character::from_model_id)
}

fn read_buddy_flags(calls: &GameCalls) -> BuddyFlags {
    BuddyFlags {
        nine_s: calls.scene_flag_is_set(buddy_flag_hash(Character::NineS)),
        two_b: calls.scene_flag_is_set(buddy_flag_hash(Character::TwoB)),
        a2: calls.scene_flag_is_set(buddy_flag_hash(Character::A2)),
        auto: calls.scene_flag_is_set(BUDDY_AUTO_FLAG_HASH),
    }
}

fn log_when_it_changed(line: String) {
    let Ok(mut last) = LAST_CHOICE_LINE.lock() else {
        return;
    };
    if *last == line {
        return;
    }
    log_within(&CHOICE_LINES, "partner choice lines", || line.clone());
    *last = line;
}

fn choose_before_the_spawn(calls: &GameCalls, hero: usize) {
    let wanted = Character::from_object_id(WANTED_OBJECT_ID.load(Ordering::Acquire));
    let slot_is_empty = slot_is_empty(calls, hero);
    let flags = read_buddy_flags(calls);
    let readings = EnginePartnerReadings {
        wanted,
        hero: character_of(calls, hero),
        slot_is_empty,
        flags,
        // The read system is asked only where the answer decides something.
        wanted_data_is_loaded: slot_is_empty
            && flags.any()
            && wanted.is_some_and(|wanted| calls.object_data_is_loaded(wanted.object_id())),
    };
    let wanted_name = wanted.map_or("nobody", Character::launcher_id);
    match choose_engine_partner(&readings) {
        EnginePartnerChoice::LeftAlone(reason) => log_when_it_changed(format!(
            "partner: the engine's own partner spawn runs (partner flags: {}; wanted: {wanted_name}); left alone: {reason}",
            flags.describe()
        )),
        EnginePartnerChoice::Steered(writes) => {
            for write in &writes {
                match write {
                    BuddyFlagWrite::Set(character) => calls.set_scene_flag(buddy_flag(*character)),
                    BuddyFlagWrite::Clear(character) => {
                        calls.clear_scene_flag(buddy_flag(*character));
                    }
                }
            }
            log_when_it_changed(format!(
                "partner: the engine's own partner spawn runs (partner flags: {}; wanted: {wanted_name}); the flags now say {}, so the engine itself makes {wanted_name}",
                flags.describe(),
                read_buddy_flags(calls).describe()
            ));
        }
    }
}

fn note_what_the_engine_made(calls: &GameCalls, hero: usize) {
    let buddy = calls.current_buddy();
    if buddy == 0 || slot_is_empty(calls, hero) {
        return;
    }
    let model_id = calls.read_u32_at(calls.body_field(buddy, BODY_MODEL_ID_OFFSET));
    log_when_it_changed(format!(
        "partner: the engine made its partner: {} (object {})",
        model_id
            .as_ref()
            .ok()
            .and_then(|id| Character::from_model_id(*id))
            .map_or("another body", Character::launcher_id),
        model_id.map_or("unread".to_owned(), |id| format!("{id:#X}"))
    ));
}

#[expect(
    unsafe_code,
    reason = "calls the original partner spawn through the trampoline MinHook built"
)]
extern "system" fn engine_partner_spawn_detour(hero: usize) {
    let calls = CHOICE_CALLS.get().filter(|_| hero != 0);
    if let Some(calls) = calls {
        let _ = panic::catch_unwind(|| choose_before_the_spawn(calls, hero));
    }
    let original = ORIGINAL_ENGINE_PARTNER_SPAWN.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original spawn with the hero the game passed.
    unsafe { core::mem::transmute::<*mut c_void, EnginePartnerSpawn>(original)(hero) };
    if let Some(calls) = calls {
        let _ = panic::catch_unwind(|| note_what_the_engine_made(calls, hero));
    }
}

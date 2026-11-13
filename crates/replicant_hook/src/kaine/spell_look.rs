use core::cell::Cell;
use std::sync::{LazyLock, Mutex, PoisonError};

use replicant_game::effects::{finish_effect, read_effect_look, write_effect_look};
use replicant_game::kaine::{
    SWAPPED_SPELL_EFFECT_MAX_LIFE_MS, SpellLook, SpellLookLog, SwappedSpellEffects,
    read_spell_effect_name, sent_shot_effect_ids, sent_spell_effect_name, spell_look_line,
    swapped_spell_look, tint_style,
};
use replicant_proto::PartnerBody;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::player_body::real_body;

static LOOK_LOG: LazyLock<Mutex<SpellLookLog>> = LazyLock::new(Mutex::default);
static HER_EFFECTS: LazyLock<Mutex<SwappedSpellEffects>> = LazyLock::new(Mutex::default);

thread_local! {
    static PLAYING_FOR_PARTNER: Cell<bool> = const { Cell::new(false) };
}

/// Runs `call` as playback of the partner's effect: nothing spawned inside takes her look.
pub(crate) fn play_for_partner<R>(call: impl FnOnce() -> R) -> R {
    let was = PLAYING_FOR_PARTNER.with(|flag| flag.replace(true));
    let result = call();
    PLAYING_FOR_PARTNER.with(|flag| flag.set(was));
    result
}

pub(crate) fn wears_swapped_body() -> bool {
    real_body().is_some_and(|real| real.by_swap && real.body == PartnerBody::SeedKaineE)
}

/// The look of an effect about to be spawned by name on this side, logged once per name.
pub(crate) fn spawn_look(build: &VerifiedBuild, name: u64) -> SpellLook {
    if PLAYING_FOR_PARTNER.with(Cell::get) || !wears_swapped_body() {
        return SpellLook::AsItIs;
    }
    let memory = InProcessMemory::new(build);
    let Ok(Some(name)) = read_spell_effect_name(&memory, name) else {
        return SpellLook::AsItIs;
    };
    let look = swapped_spell_look(&name, true, false);
    let first = LOOK_LOG
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_first(&name);
    if first {
        log_line(&spell_look_line(&name, look));
    }
    look
}

/// After the spawn: her effect is watched for its life, a spell effect without a pair takes
/// her colour when nothing else will colour it.
pub(crate) fn after_spawn(build: &VerifiedBuild, look: SpellLook, handle: u32, outermost: bool) {
    match look {
        SpellLook::Hers(_) => HER_EFFECTS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note(handle, net_clock_ms()),
        SpellLook::Tinted if outermost => {
            let mut memory = InProcessMemory::new(build);
            if let Ok(Some(mut effect_look)) = read_effect_look(&memory, handle) {
                tint_style(&mut effect_look.style);
                let _ = write_effect_look(&mut memory, handle, &effect_look);
            }
        }
        SpellLook::Tinted | SpellLook::AsItIs => {}
    }
}

pub(crate) fn sent_effect_name(name: String) -> String {
    sent_spell_effect_name(name, wears_swapped_body())
}

pub(crate) fn sent_shot_ids<const SLOTS: usize>(effect_ids: [i32; SLOTS]) -> [i32; SLOTS] {
    sent_shot_effect_ids(effect_ids, wears_swapped_body())
}

pub(crate) fn end_overdue_spell_effects(build: &VerifiedBuild) {
    let overdue = HER_EFFECTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take_overdue(net_clock_ms());
    if overdue.is_empty() {
        return;
    }
    let mut memory = InProcessMemory::new(build);
    let ended = overdue
        .iter()
        .filter(|handle| finish_effect(&mut memory, **handle).unwrap_or(false))
        .count();
    if ended > 0 {
        log_line(&format!(
            "spell look: ended {ended} of her effects still running after {SWAPPED_SPELL_EFFECT_MAX_LIFE_MS} ms"
        ));
    }
}

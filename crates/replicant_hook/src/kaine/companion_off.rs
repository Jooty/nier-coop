use std::cell::OnceCell;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use replicant_game::cutscene_skip::SKIP_PROMPT_UI_RVA as HUD_OBJECT_RVA;
use replicant_game::enemies::{AttackRecord, read_boss_bar_open, read_group_count};
use replicant_game::kaine::{
    COMPANION_GAUGE_MODE_OFFSET, COMPANION_LENT_AFTER_A_CALL_MS, COMPANION_LENT_AFTER_A_SCENE_MS,
    COMPANION_LENT_AFTER_HER_SCRIPT_MODE_MS, CompanionLendReason, CompanionOff,
    CompanionOffBossSigns, CompanionOffEvent, CompanionOffStep, CompanionOffTick,
    FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS, HUD_PARTNER_GAUGE_OFFSET,
    PARTNER_GAUGE_ACTOR_HANDLE_OFFSET, StandInSide,
};
use replicant_game::marker::read_world_flags;
use replicant_game::offsets::{
    ACTOR_SELF_HANDLE_OFFSET, PARTNER_ACTIVE_FLAG_OFFSET, PARTNER_KAINE_ACTOR_RVA,
    PARTNER_SCRIPT_MODE_OFFSET,
};
use replicant_game::story_progress::StoryArrays;
use replicant_game::{GameMemory, Replicant};
use replicant_proto::StoryArray;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::kaine::companion_rows_from_scripts;
use crate::kaine::stand_in::session_side;
use crate::memory::InProcessMemory;
use crate::net::net_clock_ms;
use crate::story_progress::{read_live_arrays, with_host_story};

const COMPANION_OFF_LINE_INTERVAL: Duration = Duration::from_secs(1);

static RULE: Mutex<CompanionOff> = Mutex::new(CompanionOff::new());
static KEEPS_HER_OFF: AtomicBool = AtomicBool::new(false);
static GUEST_IN_SESSION: AtomicBool = AtomicBool::new(false);
static HITS_SHE_LANDED: AtomicU32 = AtomicU32::new(0);
static LAST_LINE: Mutex<Option<Instant>> = Mutex::new(None);

/// Whether she is kept off wherever a guest is in the session (`companion_off_everywhere` in
/// nier-coop.toml, on when unset), or only in the listed fight windows.
pub(crate) fn set_companion_off_everywhere(enabled: bool) {
    RULE.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .keep_her_off_everywhere(enabled);
    log_line(&format!(
        "companion off everywhere: {} ({}; companion_off_everywhere in nier-coop.toml)",
        if enabled { "on" } else { "off" },
        if enabled {
            format!(
                "she is kept inactive wherever a guest is in the session, lent back for event scenes, \
                 her script mode and the {} script functions that drive her",
                FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS.len()
            )
        } else {
            "she is kept inactive only in the listed fights".to_owned()
        }
    ));
}

/// A script function ran on this game (popped or traced): it may open or close a fight window
/// in which the companion is kept off.
pub(crate) fn companion_off_function_ran(build: &VerifiedBuild, name: &str) {
    let mut memory = InProcessMemory::new(build);
    let pack = Replicant::new(InProcessMemory::new(build))
        .current_phase()
        .ok();
    let mut rule = RULE.lock().unwrap_or_else(PoisonError::into_inner);
    log_event(rule.use_rows_from_scripts(companion_rows_from_scripts()));
    if let Some(pack) = pack {
        log_event(rule.enter_pack(&pack));
    }
    let host_story = OnceCell::new();
    log_event(rule.function_ran_in_story(name, |id| {
        host_story
            .get_or_init(|| story_flags_of_the_host(build))
            .as_ref()
            .is_some_and(|arrays| arrays.flag(StoryArray::GameFlags, id))
    }));
    KEEPS_HER_OFF.store(rule.keeps_her_off(), Ordering::Relaxed);
    let actor = memory.module_base() + u64::from(PARTNER_KAINE_ACTOR_RVA);
    let Ok(found) = memory.read_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET) else {
        return;
    };
    let tick = CompanionOffTick {
        guest_in_session: GUEST_IN_SESSION.load(Ordering::Relaxed),
        active_now: found != 0,
        event_scene: read_world_flags(&memory).is_ok_and(|flags| flags.event_scene),
        her_script_mode: her_script_mode(&memory, actor),
        now_ms: net_clock_ms(),
    };
    let Some(lend) = rule.function_addresses_her(name, tick) else {
        return;
    };
    drop(rule);
    let written = lend.write_active
        && found == 0
        && memory
            .write_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET, 1)
            .is_ok();
    log_line(&format!(
        "companion off ({}): {name} addresses her, she is given back for it and the {COMPANION_LENT_AFTER_A_CALL_MS} ms after (active byte found {found}, {})",
        lend.fight,
        if written {
            "written 1"
        } else {
            "left as found"
        }
    ));
}

/// The fight whose companion-off window is open on this game, if any.
pub(crate) fn companion_off_fight() -> Option<&'static str> {
    RULE.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open_fight()
}

/// Once per ghost tick: keeps the companion inactive (active byte 0, as the game's deactivation)
/// while a guest is in the session, everywhere or in listed fights only by the setting, except
/// while she is lent to a scene, her script mode or a call that addresses her.
pub(crate) fn keep_companion_off(build: &VerifiedBuild, guest_in_session: bool) {
    GUEST_IN_SESSION.store(guest_in_session, Ordering::Relaxed);
    let mut memory = InProcessMemory::new(build);
    let actor = memory.module_base() + u64::from(PARTNER_KAINE_ACTOR_RVA);
    let mut rule = RULE.lock().unwrap_or_else(PoisonError::into_inner);
    log_event(rule.use_rows_from_scripts(companion_rows_from_scripts()));
    if let Ok(pack) = Replicant::new(InProcessMemory::new(build)).current_phase() {
        log_event(rule.enter_pack(&pack));
    }
    if rule.story_flags_are_read_here()
        && let Some(arrays) = story_flags_of_the_host(build)
    {
        log_event(rule.story_flags(|id| arrays.flag(StoryArray::GameFlags, id)));
    }
    let event_scene = read_world_flags(&memory).is_ok_and(|flags| flags.event_scene);
    let now_ms = net_clock_ms();
    if let Some(group) = rule.boss_group_watched() {
        log_event(rule.boss_signs(CompanionOffBossSigns {
            gauge_open: read_boss_bar_open(&memory),
            boss_actors: read_group_count(&memory, group),
            event_scene,
            now_ms,
        }));
    }
    let Ok(found) = memory.read_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET) else {
        return;
    };
    let kept_off_before = rule.keeps_her_off();
    let step = rule.step(CompanionOffTick {
        guest_in_session,
        active_now: found != 0,
        event_scene,
        her_script_mode: her_script_mode(&memory, actor),
        now_ms,
    });
    KEEPS_HER_OFF.store(rule.keeps_her_off(), Ordering::Relaxed);
    if rule.keeps_her_off_everywhere() && rule.keeps_her_off() != kept_off_before {
        log_line(if rule.keeps_her_off() {
            "companion off (everywhere): a guest is in the session, she is kept inactive here"
        } else {
            "companion off (everywhere): no guest in the session any more, she is the game's again"
        });
    }
    let fight = rule.acting_row().unwrap_or("no fight");
    match step {
        CompanionOffStep::Deactivate => {
            if memory
                .write_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET, 0)
                .is_err()
            {
                log_line("companion off: cannot write her active byte");
            }
        }
        CompanionOffStep::GiveBack { active } => {
            if active {
                let _ = memory.write_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET, 1);
            }
            log_line(&format!(
                "companion off: given back to the scripts as they left her (active byte {})",
                u8::from(active)
            ));
        }
        CompanionOffStep::Lend { write_active, why } => {
            let written = write_active
                && memory
                    .write_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET, 1)
                    .is_ok();
            log_line(&format!(
                "companion off ({fight}): she is given back for {}, active byte found {found}, {}",
                lend_text(&why),
                if written {
                    "written 1"
                } else {
                    "left as found"
                }
            ));
        }
        CompanionOffStep::LendOver { deactivate, why } => {
            if deactivate {
                let _ = memory.write_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET, 0);
            }
            log_line(&format!(
                "companion off ({fight}): the lend for {} is over, active byte found {found}, {}",
                lend_text(&why),
                if deactivate {
                    "she is off again"
                } else {
                    "the scripts left her off"
                }
            ));
        }
        CompanionOffStep::Leave => {}
    }
    if !line_is_due() {
        return;
    }
    let Some(report) = rule.take_report() else {
        return;
    };
    let watched_group = rule.boss_group_watched();
    drop(rule);
    let left = memory
        .read_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET)
        .map_or_else(|_| "unreadable".to_owned(), |byte| byte.to_string());
    let turned_on_by = if report.turned_on_by.is_empty() {
        "nobody".to_owned()
    } else {
        report.turned_on_by.join(", ")
    };
    let lent = report
        .lent
        .as_ref()
        .map_or_else(|| "no".to_owned(), lend_text);
    let state = match (&report.lent, left.as_str()) {
        (Some(_), _) => "lent to the game",
        (None, "0") => "off",
        (None, _) => "on (turned off on the next tick)",
    };
    log_line(&format!(
        "companion off ({}): she is {state}, guest in session {guest_in_session}, active byte found {found} left {left}, \
         turned on since the last line by {turned_on_by}, lent to the game: {lent}, event scene {event_scene}, \
         hits she landed {} (must be 0 unless lent), {}{}",
        report.fight,
        HITS_SHE_LANDED.swap(0, Ordering::Relaxed),
        gauge_text(&memory, actor),
        boss_signs_text(&memory, watched_group)
    ));
}

/// Called for every hit the game hands a victim: counts the hits the companion lands while the
/// rule keeps her off, for the once-a-second line.
pub(crate) fn note_hit_for_companion_off(build: &VerifiedBuild, victim: u64, record: u64) {
    if !KEEPS_HER_OFF.load(Ordering::Relaxed) {
        return;
    }
    let memory = InProcessMemory::new(build);
    let her = memory.module_base() + u64::from(PARTNER_KAINE_ACTOR_RVA);
    if victim == her {
        return;
    }
    let Some(attack) = AttackRecord::read(&memory, record) else {
        return;
    };
    if attack.sources(&memory).contains(&Some(her)) {
        HITS_SHE_LANDED.fetch_add(1, Ordering::Relaxed);
    }
}

fn line_is_due() -> bool {
    let mut last = LAST_LINE.lock().unwrap_or_else(PoisonError::into_inner);
    let now = Instant::now();
    if last.is_some_and(|at| now.saturating_duration_since(at) < COMPANION_OFF_LINE_INTERVAL) {
        return false;
    }
    *last = Some(now);
    true
}

fn her_script_mode(memory: &InProcessMemory, actor: u64) -> bool {
    memory
        .read_i32(actor + PARTNER_SCRIPT_MODE_OFFSET)
        .is_ok_and(|mode| mode != 0)
}

fn gauge_text(memory: &InProcessMemory, actor: u64) -> String {
    let mode = memory
        .read_i32(actor + COMPANION_GAUGE_MODE_OFFSET)
        .map_or_else(|_| "unreadable".to_owned(), |mode| mode.to_string());
    let gauge_handle = memory.module_base()
        + u64::from(HUD_OBJECT_RVA)
        + HUD_PARTNER_GAUGE_OFFSET
        + PARTNER_GAUGE_ACTOR_HANDLE_OFFSET;
    let holds_her = match (
        memory.read_u64(gauge_handle),
        memory.read_u64(actor + ACTOR_SELF_HANDLE_OFFSET),
    ) {
        (Ok(gauge), Ok(hers)) => (gauge == hers).to_string(),
        _ => "unreadable".to_owned(),
    };
    // mode non-zero and her handle in the slot is how _SetKaineMode puts her gauge up; whether
    // the HUD still draws it while she is inactive is not known
    format!("life gauge: her mode {mode}, the HUD partner gauge holds her handle {holds_her}")
}

fn boss_signs_text(memory: &InProcessMemory, group: Option<u16>) -> String {
    let Some(group) = group else {
        return String::new();
    };
    let gauge =
        read_boss_bar_open(memory).map_or_else(|| "unreadable".to_owned(), |open| open.to_string());
    let actors = read_group_count(memory, group)
        .map_or_else(|| "unreadable".to_owned(), |count| count.to_string());
    format!(", boss signs: boss life gauge open {gauge}, actors in group {group} {actors}")
}

fn lend_text(why: &CompanionLendReason) -> String {
    match why {
        CompanionLendReason::Scene => {
            format!("the event scene and the {COMPANION_LENT_AFTER_A_SCENE_MS} ms after it")
        }
        CompanionLendReason::Call(name) => {
            format!("{name} and the {COMPANION_LENT_AFTER_A_CALL_MS} ms after it")
        }
        CompanionLendReason::ScriptMode(by) => format!(
            "her script mode (the scripts drive her; last script function {by}) and the \
             {COMPANION_LENT_AFTER_HER_SCRIPT_MODE_MS} ms after it"
        ),
    }
}

fn story_flags_of_the_host(build: &VerifiedBuild) -> Option<StoryArrays> {
    match session_side() {
        Some((StandInSide::Guest, _)) => with_host_story(StoryArrays::clone),
        _ => read_live_arrays(build),
    }
}

fn log_event(event: Option<CompanionOffEvent>) {
    match event {
        Some(CompanionOffEvent::Opened { fight, by }) => log_line(&format!(
            "companion off: {fight} window opened by {by}, she is kept inactive while a guest is in the session"
        )),
        Some(CompanionOffEvent::Closed { fight, by }) => {
            log_line(&format!("companion off: {fight} window closed by {by}"));
        }
        None => {}
    }
}

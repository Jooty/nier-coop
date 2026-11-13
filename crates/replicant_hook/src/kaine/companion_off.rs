use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, Instant};

use replicant_game::cutscene_skip::SKIP_PROMPT_UI_RVA as HUD_OBJECT_RVA;
use replicant_game::enemies::AttackRecord;
use replicant_game::kaine::{
    COMPANION_GAUGE_MODE_OFFSET, CompanionOff, CompanionOffEvent, CompanionOffStep,
    HUD_PARTNER_GAUGE_OFFSET, PARTNER_GAUGE_ACTOR_HANDLE_OFFSET,
};
use replicant_game::offsets::{
    ACTOR_SELF_HANDLE_OFFSET, PARTNER_ACTIVE_FLAG_OFFSET, PARTNER_KAINE_ACTOR_RVA,
};
use replicant_game::{GameMemory, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

const COMPANION_OFF_LINE_INTERVAL: Duration = Duration::from_secs(1);

static RULE: Mutex<CompanionOff> = Mutex::new(CompanionOff::new());
static WINDOW_OPEN: AtomicBool = AtomicBool::new(false);
static HITS_SHE_LANDED: AtomicU32 = AtomicU32::new(0);
static LAST_LINE: Mutex<Option<Instant>> = Mutex::new(None);

/// A script function ran on this game (popped or traced): it may open or close a fight window
/// in which the companion is kept off.
pub(crate) fn companion_off_function_ran(build: &VerifiedBuild, name: &str) {
    let pack = Replicant::new(InProcessMemory::new(build))
        .current_phase()
        .ok();
    let mut rule = RULE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Some(pack) = pack {
        log_event(rule.enter_pack(&pack));
    }
    log_event(rule.function_ran(name));
    WINDOW_OPEN.store(rule.open_fight().is_some(), Ordering::Relaxed);
}

/// The fight whose companion-off window is open on this game, if any.
pub(crate) fn companion_off_fight() -> Option<&'static str> {
    RULE.lock()
        .unwrap_or_else(PoisonError::into_inner)
        .open_fight()
}

/// Keeps the companion inactive (her active byte 0, what `_DisactivateUniqueActor(-2)` writes)
/// while a listed fight window is open and a guest is in the session; called once per ghost tick.
pub(crate) fn keep_companion_off(build: &VerifiedBuild, guest_in_session: bool) {
    let mut memory = InProcessMemory::new(build);
    let actor = memory.module_base() + u64::from(PARTNER_KAINE_ACTOR_RVA);
    let mut rule = RULE.lock().unwrap_or_else(PoisonError::into_inner);
    if let Ok(pack) = Replicant::new(InProcessMemory::new(build)).current_phase() {
        log_event(rule.enter_pack(&pack));
    }
    WINDOW_OPEN.store(rule.open_fight().is_some(), Ordering::Relaxed);
    let Ok(found) = memory.read_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET) else {
        return;
    };
    match rule.step(guest_in_session, found != 0) {
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
        CompanionOffStep::Leave => {}
    }
    if !line_is_due() {
        return;
    }
    let Some(report) = rule.take_report() else {
        return;
    };
    let left = memory
        .read_u8(actor + PARTNER_ACTIVE_FLAG_OFFSET)
        .map_or_else(|_| "unreadable".to_owned(), |byte| byte.to_string());
    let turned_on_by = if report.turned_on_by.is_empty() {
        "nobody".to_owned()
    } else {
        report.turned_on_by.join(", ")
    };
    log_line(&format!(
        "companion off ({}): guest in session {guest_in_session}, active byte found {found} left {left}, \
         turned on since the last line by {turned_on_by}, hits she landed {} (must be 0), {}",
        report.fight,
        HITS_SHE_LANDED.swap(0, Ordering::Relaxed),
        gauge_text(&memory, actor)
    ));
}

/// Called for every hit the game hands a victim: counts the hits the companion lands while a
/// window is open, for the once-a-second line.
pub(crate) fn note_hit_for_companion_off(build: &VerifiedBuild, victim: u64, record: u64) {
    if !WINDOW_OPEN.load(Ordering::Relaxed) {
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

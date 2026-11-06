use std::sync::atomic::Ordering;
use std::sync::{Mutex, PoisonError};

use rc_game::dialogue::{
    CONTROL_MASK_TALK_SLOT_RVA, DialogueTracker, MirrorAction, MirrorInput, PartnerTalkMirror,
    TALK_WINDOW_RVA, find_talk_id, read_talk_cursor,
};
use rc_game::marker::{WorldFlags, read_world_flags};
use rc_game::readiness::{classify, read_play_checks};
use rc_game::stay_together::local_phase_index;
use rc_game::{GameMemory, Replicant};
use rc_proto::{DialogueLine, DialogueUpdate, Message, PartnerActivity};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::hooks::MIRRORING;
use crate::dialogue::window_calls::{close_mirror, open_mirror, show_line};
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetThread, net_clock_ms};

static TRACKER: Mutex<DialogueTracker> = Mutex::new(DialogueTracker::new());
static MIRROR: Mutex<PartnerTalkMirror> = Mutex::new(PartnerTalkMirror::new());
static PARTNER_LINE: Mutex<Option<DialogueLine>> = Mutex::new(None);

pub fn tick_dialogue(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let partner_line = receive_partner_line(net, &view);
    let game = Replicant::new(InProcessMemory::new(build));
    let memory = game.memory();
    let base = memory.module_base();
    let window_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    let started_own_talk = memory
        .read_i32(base + u64::from(CONTROL_MASK_TALK_SLOT_RVA))
        .is_ok_and(|slot| slot != 0);
    let own_window_open = window_open && (started_own_talk || !MIRRORING.load(Ordering::Acquire));
    let own_talk_line = share_own_talk(net, &view, memory, own_window_open);
    let flags = read_world_flags(memory).ok();
    let same_zone = view
        .remote
        .as_ref()
        .is_some_and(|remote| local_phase_index(&game) == Some(remote.state.phase));
    let can_show = same_zone
        && flags.is_some_and(|flags| !flags.event_scene && !flags.world_frozen())
        && classify(read_play_checks(&game)).is_in_play();
    let mut mirror = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);
    let action = mirror.step(MirrorInput {
        partner_line,
        can_show,
        window_open,
        own_talk: own_talk_line.filter(|_| started_own_talk && own_window_open),
        now_ms: net_clock_ms(),
    });
    if let Some(action) = action {
        apply(build, action);
    }
    let window_still_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    MIRRORING.store(mirror.is_showing() && window_still_open, Ordering::Release);
}

/// This player's activity for the snapshot: a window that only mirrors the partner's talk
/// does not make this player "in dialogue".
pub fn local_activity(flags: WorldFlags) -> PartnerActivity {
    WorldFlags {
        talk_window_open: flags.talk_window_open && !MIRRORING.load(Ordering::Acquire),
        ..flags
    }
    .activity()
}

fn receive_partner_line(net: &NetThread, view: &NetShared) -> Option<DialogueLine> {
    let mut partner_line = PARTNER_LINE.lock().unwrap_or_else(PoisonError::into_inner);
    for message in net.take_relayed(|message| matches!(message, Message::Dialogue(_))) {
        match message {
            Message::Dialogue(DialogueUpdate::Line(line)) => {
                log_line(&format!(
                    "dialogue: partner on talk {} line {}",
                    line.talk_id, line.line
                ));
                *partner_line = Some(line);
            }
            Message::Dialogue(DialogueUpdate::Ended) => {
                log_line("dialogue: partner talk ended");
                *partner_line = None;
            }
            _ => {}
        }
    }
    if !view.partner_in_play() {
        *partner_line = None;
    }
    *partner_line
}

fn share_own_talk(
    net: &NetThread,
    view: &NetShared,
    memory: &InProcessMemory,
    own_window_open: bool,
) -> Option<DialogueLine> {
    let cursor = read_talk_cursor(memory)
        .ok()
        .flatten()
        .filter(|_| own_window_open);
    let mut tracker = TRACKER.lock().unwrap_or_else(PoisonError::into_inner);
    let update = tracker.observe(cursor, view.partner_in_play(), |talk_address| {
        let talk_id = find_talk_id(memory, talk_address).ok().flatten();
        if talk_id.is_none() {
            log_line("dialogue: the open talk is not in the talk table, not shared");
        }
        talk_id
    });
    if let Some(update) = update {
        match update {
            DialogueUpdate::Line(line) => log_line(&format!(
                "dialogue: sharing talk {} line {}",
                line.talk_id, line.line
            )),
            DialogueUpdate::Ended => log_line("dialogue: sharing talk ended"),
        }
        net.send_relayed(Message::Dialogue(update));
    }
    tracker.current_line()
}

fn apply(build: &VerifiedBuild, action: MirrorAction) {
    match action {
        MirrorAction::Open(line) => {
            let opened = open_mirror(build, line);
            log_line(&format!(
                "dialogue: mirror open talk {} at line {} ({})",
                line.talk_id,
                line.line,
                if opened { "shown" } else { "the game refused" }
            ));
        }
        MirrorAction::ShowLine(line) => {
            show_line(&mut InProcessMemory::new(build), line);
            log_line(&format!("dialogue: mirror line {line}"));
        }
        MirrorAction::Close => {
            close_mirror(build);
            log_line("dialogue: mirror closed");
        }
        MirrorAction::JumpOwnTalk(line) => {
            show_line(&mut InProcessMemory::new(build), line);
            log_line(&format!(
                "dialogue: own talk joined the partner's at line {line}"
            ));
        }
    }
}

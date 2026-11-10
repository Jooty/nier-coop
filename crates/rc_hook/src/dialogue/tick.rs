use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, PoisonError};

use rc_game::dialogue::{
    DialogueTracker, MirrorAction, MirrorInput, PartnerTalkMemory, PartnerTalkMirror, TALK_WINDOW_RVA, TalkLeader,
    WATCHER_HOLD_MASK, WatcherHold, WatcherHoldAction, WatcherHoldInput, find_talk_id,
    own_talk_started, read_talk_cursor, within_talk_range,
};
use rc_game::marker::{WorldFlags, read_world_flags};
use rc_game::readiness::{classify, read_play_checks};
use rc_game::stay_together::{FollowRole, local_phase_index};
use rc_game::{GameMemory, Replicant};
use rc_proto::{DialogueLine, DialogueUpdate, Message, PartnerActivity};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::dialogue::hooks::{FOLLOWING_HOST_TALK, MIRRORING};
use crate::dialogue::tags::MirrorActionGuard;
use crate::dialogue::talker_name::begin_talk;
use crate::dialogue::watcher_hold::{
    forget_mirror_wrote_slot_one, read_slot_one, slot_one_written_by_mirror,
    take_other_code_wrote_slot_one, write_slot_one,
};
use crate::dialogue::window_calls::{close_mirror, open_mirror, show_line};
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetThread, net_clock_ms};

static TRACKER: Mutex<DialogueTracker> = Mutex::new(DialogueTracker::new());
static MIRROR: Mutex<PartnerTalkMirror> = Mutex::new(PartnerTalkMirror::new());
static OWN_TALK_WINDOW_WAS_OPEN: AtomicBool = AtomicBool::new(false);
static PARTNER_TALK: Mutex<PartnerTalkMemory> = Mutex::new(PartnerTalkMemory::new());
static HOLD: Mutex<WatcherHold> = Mutex::new(WatcherHold::new());

pub fn tick_dialogue(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        release_hold_without_link(build);
        return;
    };
    let view = net.view();
    crate::dialogue::choices::receive_partner_choices(net);
    receive_partner_lines(net);
    let game = Replicant::new(InProcessMemory::new(build));
    let memory = game.memory();
    let base = memory.module_base();
    let window_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    let slot_one_mask = read_slot_one(build);
    let taken_over = take_other_code_wrote_slot_one();
    let holding = HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_holding();
    let started_own_talk = own_talk_started(
        slot_one_mask,
        slot_one_written_by_mirror(),
        holding,
        taken_over,
    );
    let own_window_open = window_open && (started_own_talk || !MIRRORING.load(Ordering::Acquire));
    if OWN_TALK_WINDOW_WAS_OPEN.swap(own_window_open, Ordering::Relaxed) != own_window_open {
        log_line(&format!(
            "dialogue: your talk window {} (own talk started: {started_own_talk})",
            if own_window_open { "opened" } else { "closed" }
        ));
    }
    let own_talk_line = share_own_talk(net, &view, memory, own_window_open);
    let flags = read_world_flags(memory).ok();
    let mut mirror = MIRROR.lock().unwrap_or_else(PoisonError::into_inner);
    let in_range = view.remote.as_ref().is_some_and(|remote| {
        local_phase_index(&game) == Some(remote.state.phase)
            && game.position().is_ok_and(|local| {
                let [x, y, z] = remote.state.position;
                let distance =
                    ((local.x - x).powi(2) + (local.y - y).powi(2) + (local.z - z).powi(2)).sqrt();
                within_talk_range(distance, mirror.is_showing())
            })
    });
    let can_show = in_range
        && flags.is_some_and(|flags| {
            !flags.event_scene && !flags.main_menu_open && !flags.world_frozen()
        })
        && classify(read_play_checks(&game)).is_in_play();
    let leader = match FollowRole::from_slot(view.local_slot) {
        _ if !in_range || !view.partner_in_play() => TalkLeader::Either,
        FollowRole::Host => TalkLeader::Local,
        FollowRole::Guest => TalkLeader::Partner,
        FollowRole::Unknown => TalkLeader::Either,
    };
    let own_talk = own_talk_line.filter(|_| started_own_talk && own_window_open);
    let now_ms = net_clock_ms();
    let (partner_line, partner_ended_talk) = {
        let memory = PARTNER_TALK.lock().unwrap_or_else(PoisonError::into_inner);
        (
            memory.for_follow(own_talk, leader),
            memory.ended_for_follow(own_talk, leader, now_ms),
        )
    };
    let action = mirror.step(MirrorInput {
        partner_line,
        partner_ended_talk,
        can_show,
        window_open,
        own_talk,
        leader,
        now_ms,
    });
    if let Some(action) = action {
        apply(build, action);
    }
    FOLLOWING_HOST_TALK.store(
        own_talk.is_some() && mirror.holds_own_talk(),
        Ordering::Release,
    );
    let window_still_open = memory
        .read_i32(base + u64::from(TALK_WINDOW_RVA))
        .is_ok_and(|state| state != 0);
    let mirror_showing = mirror.is_showing() && window_still_open;
    if !window_still_open {
        forget_mirror_wrote_slot_one();
    }
    MIRRORING.store(mirror_showing, Ordering::Release);
    drop(mirror);
    let hold_action = HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .step(WatcherHoldInput {
            mirror_showing,
            slot_one_mask,
            taken_over,
        });
    apply_hold(build, hold_action, view.peer_name.as_deref());
}

fn release_hold_without_link(build: &VerifiedBuild) {
    PARTNER_TALK
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .forget();
    FOLLOWING_HOST_TALK.store(false, Ordering::Release);
    let action = HOLD
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .release_on_link_loss(read_slot_one(build));
    apply_hold(build, action, None);
}

fn apply_hold(build: &VerifiedBuild, action: Option<WatcherHoldAction>, partner: Option<&str>) {
    let partner = partner.unwrap_or("your partner");
    match action {
        Some(WatcherHoldAction::Hold) => {
            if !write_slot_one(build, WATCHER_HOLD_MASK) {
                HOLD.lock()
                    .unwrap_or_else(PoisonError::into_inner)
                    .abandon();
                log_line(
                    "dialogue: could not hold you during the partner's talk, the game's mask setter is not hooked",
                );
                return;
            }
            log_line(&format!("dialogue: holding you during {partner}'s talk"));
        }
        Some(WatcherHoldAction::Release) => {
            write_slot_one(build, 0);
            log_line(&format!("dialogue: released from {partner}'s talk"));
        }
        None => {}
    }
}

pub(super) fn own_talk_line() -> Option<DialogueLine> {
    TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .current_line()
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

fn receive_partner_lines(net: &NetThread) {
    for message in net.take_relayed(|message| matches!(message, Message::Dialogue(_))) {
        let Message::Dialogue(update) = message else {
            continue;
        };
        PARTNER_TALK
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .receive(update, net_clock_ms());
        match update {
            DialogueUpdate::Line(line) => log_line(&format!(
                "dialogue: partner on talk {} line {}",
                line.talk_id, line.line
            )),
            DialogueUpdate::Ended => log_line("dialogue: partner talk ended"),
        }
    }
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
        MirrorAction::FollowStarted(talk_id) => {
            log_line(&format!(
                "dialogue: the host leads talk {talk_id}, your copy waits on the host's lines"
            ));
            return;
        }
        MirrorAction::FollowLine(line) => {
            // The guest's own copy of the talk: its line tags run here as the script's own.
            show_line(&mut InProcessMemory::new(build), line);
            log_line(&format!(
                "dialogue: your copy follows the host to line {line}"
            ));
            return;
        }
        MirrorAction::FinishOwnTalk(talk_id) => {
            close_mirror(build);
            log_line(&format!(
                "dialogue: the host left talk {talk_id}, your copy is closed so your game moves on with the host"
            ));
            return;
        }
        MirrorAction::FollowGaveUp(talk_id) => {
            log_line(&format!(
                "dialogue: the host never showed talk {talk_id}, yours goes on by your own buttons"
            ));
            return;
        }
        MirrorAction::SkipEndedTalk(talk_id) => {
            log_line(&format!(
                "dialogue: not mirroring talk {talk_id}, you just finished it (the partner's is not shown back)"
            ));
            return;
        }
        _ => {}
    }
    let _skip_tags = MirrorActionGuard::begin();
    match action {
        MirrorAction::Open(line) => {
            begin_talk();
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
        MirrorAction::FollowStarted(_)
        | MirrorAction::FollowLine(_)
        | MirrorAction::FinishOwnTalk(_)
        | MirrorAction::FollowGaveUp(_)
        | MirrorAction::SkipEndedTalk(_) => {}
    }
}

use std::sync::PoisonError;
use std::sync::atomic::Ordering;

use rc_game::boss_messages::{GuestFacts, GuestVerdict, SkipReason, guest_verdict};
use rc_game::readiness::{classify, read_play_checks};
use rc_game::stay_together::FollowRole;
use rc_game::{PhaseEntry, Replicant};
use rc_proto::Message;

use crate::boss_messages::detour::raise_through_original;
use crate::boss_messages::state::{
    CountedAs, LAST_SENT_SEQ, POST_PAIRS, RAISED_SEQS, count, post_clock_ms,
};
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, NetState};
use crate::story_relay::shared_story_events;

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::ScriptMessage(_)));
    let view = net.view();
    if view.state != NetState::Connected {
        LAST_SENT_SEQ.store(0, Ordering::Relaxed);
        RAISED_SEQS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
        POST_PAIRS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clear();
    }
    if arrived.is_empty() {
        return;
    }
    let is_guest = view.state == NetState::Connected
        && FollowRole::from_slot(view.local_slot) == FollowRole::Guest;
    let game = Replicant::new(InProcessMemory::new(build));
    let local_in_play = classify(read_play_checks(&game)).is_in_play();
    let local_phase = current_phase_index(&game);
    for message in arrived {
        let Message::ScriptMessage(arrived) = message else {
            continue;
        };
        let verdict = guest_verdict(&GuestFacts {
            enabled: shared_story_events(),
            is_guest,
            local_in_play,
            local_phase,
            message_phase: arrived.phase,
            message: arrived.message,
        });
        match verdict {
            GuestVerdict::Skip(reason) => {
                count(CountedAs::Skipped);
                log_line(&format!(
                    "boss message: {} for phase {} not raised ({})",
                    arrived.message,
                    arrived.phase,
                    skip_text(reason)
                ));
            }
            GuestVerdict::Raise => {
                crate::boss_messages::fight::raise(arrived.phase);
                raise_once(build, arrived.message, arrived.seq);
            }
        }
    }
}

fn raise_once(build: &VerifiedBuild, message: u32, seq: u32) {
    let allowed = RAISED_SEQS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .allow(seq);
    if !allowed {
        count(CountedAs::Skipped);
        log_line(&format!(
            "boss message: number {seq} ({message}) already raised, skipped"
        ));
        return;
    }
    if raise_through_original(build, message) {
        count(CountedAs::Raised);
        log_line(&format!(
            "boss message: guest raised {message} (number {seq})"
        ));
        let gap = POST_PAIRS
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .note_raised(message, post_clock_ms());
        if let Some(gap) = gap {
            log_line(&format!(
                "boss message: {message} was posted by your game and by the host {gap} ms apart"
            ));
        }
    } else {
        count(CountedAs::Skipped);
        log_line(&format!(
            "boss message: {message} not raised (hook not ready)"
        ));
    }
}

pub(super) fn current_phase_index(game: &Replicant<InProcessMemory>) -> Option<u16> {
    let phase = game.current_phase().ok()?;
    PhaseEntry::index_of(&phase).and_then(|index| u16::try_from(index).ok())
}

fn skip_text(reason: SkipReason) -> &'static str {
    match reason {
        SkipReason::Disabled => "shared story events are off",
        SkipReason::NotGuest => "this machine is not the guest",
        SkipReason::NotInPlay => "not in play",
        SkipReason::OtherPhase => "another phase",
        SkipReason::NotMirrored => "outside the mirrored numbers",
    }
}

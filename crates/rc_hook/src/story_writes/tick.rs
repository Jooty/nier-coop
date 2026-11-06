use rc_game::stay_together::FollowRole;
use rc_game::story_writes::host_write_outcome;
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::net::{self, NetState};
use crate::story_progress::{apply_change, describe_changes, read_live_arrays};
use crate::story_writes::calls::{send_talker_writes, talk_callback_running};

pub fn tick(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let arrived = net.take_relayed(|message| matches!(message, Message::StoryWrite(_)));
    if view.state != NetState::Connected {
        return;
    }
    match FollowRole::from_slot(view.local_slot) {
        FollowRole::Guest => send_talker_writes(build, talk_callback_running()),
        FollowRole::Host => apply_guest_writes(build, arrived),
        FollowRole::Unknown => {}
    }
}

fn apply_guest_writes(build: &VerifiedBuild, arrived: Vec<Message>) {
    for message in arrived {
        let Message::StoryWrite(write) = message else {
            continue;
        };
        let Some(live) = read_live_arrays(build) else {
            log_line("talk-write: host could not read its story state, write dropped");
            continue;
        };
        let outcome = host_write_outcome(&live, &write.changes);
        for &change in &outcome.apply {
            apply_change(build, change);
        }
        log_line(&format!(
            "talk-write: host applied {} changes, {} already held: {}",
            outcome.apply.len(),
            outcome.already_held,
            describe_changes(&outcome.apply)
        ));
    }
}

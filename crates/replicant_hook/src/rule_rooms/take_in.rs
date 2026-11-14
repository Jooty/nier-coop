use std::time::Duration;

use replicant_game::Replicant;
use replicant_game::rule_rooms::{RuleRoom, TakeIn, take_in_failed_line};
use replicant_game::stay_together::PARTNER_TIMEOUT_MS;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{NetShared, NetSharedExt};

pub(super) fn take_the_guest_in(build: &VerifiedBuild, view: &NetShared, room: &RuleRoom) {
    let game = Replicant::new(InProcessMemory::new(build));
    let guest_at = game.position().ok().map(|at| [at.x, at.y, at.z]);
    let host_at = view
        .partner_state()
        .filter(|_| view.partner_in_play())
        .filter(|remote| remote.age() <= Duration::from_millis(PARTNER_TIMEOUT_MS))
        .map(|remote| remote.state.position);
    let take_in = TakeIn::judge(guest_at, host_at);
    if let TakeIn::MoveBeside { host_at, .. } = take_in
        && let Err(reason) = crate::teleport::move_beside(build, host_at)
    {
        log_line(&take_in_failed_line(room, &reason));
        return;
    }
    log_line(&take_in.line(room));
}

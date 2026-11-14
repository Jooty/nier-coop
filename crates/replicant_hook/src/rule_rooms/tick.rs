use crate::build_gate::VerifiedBuild;
use crate::rule_rooms::{breaker_return, clock_log, rearm, receive};

pub fn tick(build: &VerifiedBuild) {
    breaker_return::tick(build);
    receive::tick(build);
    rearm::tick(build);
    clock_log::tick();
}

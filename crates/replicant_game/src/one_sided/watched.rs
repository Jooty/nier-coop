use crate::one_sided::{Side, Verdict, WATCHED_CALLBACK_TABLE};
use crate::stay_together::FollowRole;
use crate::story_calls::name_hash;

pub fn is_watched_callback(pack_hash: u64, callback: &str) -> bool {
    WATCHED_CALLBACK_TABLE
        .binary_search(&(pack_hash, name_hash(callback)))
        .is_ok()
}

pub fn one_sided_log_line(verdict: &Verdict, local_role: FollowRole, zone: &str) -> String {
    match verdict {
        Verdict::BothRan { name } => format!("both ran: {name}"),
        Verdict::OneSided { name, ran_on, .. } => {
            let ran_on_role = match (ran_on, local_role) {
                (Side::Here, role) | (Side::Partner, role @ FollowRole::Unknown) => role,
                (Side::Partner, FollowRole::Host) => FollowRole::Guest,
                (Side::Partner, FollowRole::Guest) => FollowRole::Host,
            };
            let who = match ran_on_role {
                FollowRole::Host => "host",
                FollowRole::Guest => "guest",
                FollowRole::Unknown => "one game",
            };
            format!("one-sided: {name} ran on {who} only ({zone}, 10 s)")
        }
    }
}

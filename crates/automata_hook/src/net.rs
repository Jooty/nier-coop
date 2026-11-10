mod automata_net;
mod co_op_view;
mod link_state;
mod load_signal;
mod local_play;

#[cfg(windows)]
mod global;

pub use automata_net::AutomataNet;
pub use co_op_view::{co_op_status_of, link_not_started_status};
pub use link_state::{AutomataLinkState, NetShared};
pub use load_signal::{
    HeardLoadSignal, is_snapshot_without_body, local_load_signal, partner_load_of,
    set_local_load_signal, snapshot_without_body,
};
pub use local_play::{ReadinessPacer, local_in_play, readiness_of, set_local_in_play};

#[cfg(windows)]
pub use global::{co_op_status, handle, start_link};

const LINK_NOT_STARTED_TEXT: &str = "not started (the launcher gave no link port)";
const UNNAMED_PARTNER_TEXT: &str = "partner";
const READINESS_HEARTBEAT_MS: u64 = 1_000;

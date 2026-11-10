mod automata_net;
mod co_op_view;
mod link_state;

#[cfg(windows)]
mod global;

pub use automata_net::AutomataNet;
pub use co_op_view::{co_op_status_of, link_not_started_status};
pub use link_state::{AutomataLinkState, NetShared};

#[cfg(windows)]
pub use global::{co_op_status, handle, start_link};

const LINK_NOT_STARTED_TEXT: &str = "not started (the launcher gave no link port)";
const UNNAMED_PARTNER_TEXT: &str = "partner";

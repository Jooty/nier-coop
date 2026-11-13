mod automata_net;
mod co_op_view;
mod flight_report;
mod link_state;
mod load_signal;
mod local_play;
mod partner_watch;
mod playing_event;
mod session_mode_view;
mod shown_players;
mod story_place;

#[cfg(windows)]
mod global;

pub use automata_net::AutomataNet;
pub use co_op_view::{co_op_status_of, link_not_started_status};
pub use flight_report::{local_flight_report, set_local_flight_report};
pub use link_state::{AutomataLinkState, NetShared};
pub use load_signal::{
    HeardLoadSignal, is_snapshot_without_body, local_load_signal, partner_load_of,
    partner_place_of, player_load_of, set_local_load_signal, snapshot_without_body,
};
pub use local_play::{ReadinessPacer, local_in_play, readiness_of, set_local_in_play};
pub use partner_watch::{NetSharedExt, PartnerWatch};
pub use playing_event::{local_playing_event, partner_playing_event_of, set_local_playing_event};
pub use session_mode_view::SessionModeExt;
pub use shown_players::ShownPlayersExt;
pub use story_place::{local_story_place, set_local_story_place};

#[cfg(windows)]
pub use global::{co_op_status, current_session_mode, handle, start_link};

const LINK_NOT_STARTED_TEXT: &str = "not started (the launcher gave no link port)";
const UNNAMED_PARTNER_TEXT: &str = "partner";
const READINESS_HEARTBEAT_MS: u64 = 1_000;

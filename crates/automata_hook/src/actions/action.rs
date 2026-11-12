use automata_game::travel::TravelPoint;
use nc_hook::diagnostics::log_line;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    LogLine(String),
    GoToPartner,
    GoToPlayer(u8),
    TravelTo(TravelPoint),
}

impl Action {
    pub(super) fn run(self) {
        match self {
            Self::LogLine(text) => log_line(&text),
            Self::GoToPartner => crate::travel_driver::run_go_to_partner(),
            Self::GoToPlayer(slot) => crate::travel_driver::run_go_to_player(slot),
            Self::TravelTo(point) => crate::travel_driver::run_local_jump(point),
        }
    }
}

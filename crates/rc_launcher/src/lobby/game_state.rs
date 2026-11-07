use rc_proto::GameReadiness;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameState {
    NotStarted,
    Starting,
    WaitingForPartner,
    InPlay,
    Closed,
}

#[derive(Debug, Clone, Copy)]
pub struct OwnGameInputs {
    pub started: bool,
    pub closed: bool,
    pub waiting_for_host_game: bool,
    pub own: GameReadiness,
    pub partner: GameReadiness,
}

impl GameState {
    pub fn text(self) -> &'static str {
        match self {
            Self::NotStarted => "not started",
            Self::Starting => "starting",
            Self::WaitingForPartner => "waiting for partner",
            Self::InPlay => "in play",
            Self::Closed => "closed",
        }
    }
}

pub fn own_game_state(inputs: OwnGameInputs) -> GameState {
    if inputs.closed {
        return GameState::Closed;
    }
    if !inputs.started {
        return GameState::NotStarted;
    }
    if inputs.waiting_for_host_game {
        return GameState::WaitingForPartner;
    }
    match inputs.own {
        GameReadiness::InPlay => GameState::InPlay,
        GameReadiness::Loading if inputs.partner == GameReadiness::NotLinked => {
            GameState::WaitingForPartner
        }
        GameReadiness::Loading | GameReadiness::NotLinked => GameState::Starting,
    }
}

pub fn partner_game_state(partner: GameReadiness, was_linked_before: bool) -> GameState {
    match partner {
        GameReadiness::InPlay => GameState::InPlay,
        GameReadiness::Loading => GameState::Starting,
        GameReadiness::NotLinked if was_linked_before => GameState::Closed,
        GameReadiness::NotLinked => GameState::NotStarted,
    }
}

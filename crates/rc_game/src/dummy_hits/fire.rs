use crate::stay_together::FollowRole;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DummyHitFireFacts {
    pub registered: bool,
    pub injected: bool,
    pub enabled: bool,
    pub connected: bool,
    pub partner_here: bool,
    pub args_relayable: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyHitFire {
    NotADummyHit,
    RunForPartner,
    RunAndSend,
    RunHereOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyHitOrigin {
    Here,
    Host,
    Guest,
}

impl DummyHitOrigin {
    pub fn partner_of(local: FollowRole) -> Self {
        match local {
            FollowRole::Host => Self::Guest,
            FollowRole::Guest => Self::Host,
            FollowRole::Unknown => Self::Here,
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            Self::Here => "here",
            Self::Host => "host",
            Self::Guest => "guest",
        }
    }
}

pub fn dummy_hit_fire(facts: &DummyHitFireFacts) -> DummyHitFire {
    if !facts.registered {
        return DummyHitFire::NotADummyHit;
    }
    if facts.injected {
        return DummyHitFire::RunForPartner;
    }
    if facts.enabled && facts.connected && facts.partner_here && facts.args_relayable {
        return DummyHitFire::RunAndSend;
    }
    DummyHitFire::RunHereOnly
}

pub fn dummy_hit_log_line(callback: &str, origin: DummyHitOrigin, runs_here: u32) -> String {
    format!(
        "dummy hit: {callback} from {}, runs here {runs_here}",
        origin.describe()
    )
}

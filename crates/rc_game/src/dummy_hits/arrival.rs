#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DummyHitArrivalFacts {
    pub enabled: bool,
    pub local_in_play: bool,
    pub local_phase: Option<u16>,
    pub call_phase: u16,
    pub registered_here: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyHitDrop {
    Disabled,
    NotInPlay,
    WrongPhase,
    NoDummyHere,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DummyHitArrival {
    Inject,
    Drop(DummyHitDrop),
}

pub fn dummy_hit_arrival(facts: &DummyHitArrivalFacts) -> DummyHitArrival {
    if !facts.enabled {
        return DummyHitArrival::Drop(DummyHitDrop::Disabled);
    }
    if !facts.local_in_play {
        return DummyHitArrival::Drop(DummyHitDrop::NotInPlay);
    }
    if facts.local_phase != Some(facts.call_phase) {
        return DummyHitArrival::Drop(DummyHitDrop::WrongPhase);
    }
    if !facts.registered_here {
        return DummyHitArrival::Drop(DummyHitDrop::NoDummyHere);
    }
    DummyHitArrival::Inject
}

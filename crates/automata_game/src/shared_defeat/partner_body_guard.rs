use crate::defeat::LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY;
use crate::shared_defeat::{
    BODY_FLIGHT_DEFEAT_STATE, BODY_STATE_THAT_ASKS_FOR_A_DEFEAT_AT_NO_HEALTH,
    FLIGHT_DEFEAT_TIMER_THAT_NEVER_ASKS,
};

/// The words of the held partner body that the two defeat requests without a player test read.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PartnerBodyReading {
    pub state: i32,
    pub flight_defeat_timer: f32,
    pub health: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PartnerBodyGuardAct {
    Nothing,
    /// In flight state 0x91 the game asks for this copy's defeat flow when its timer runs from
    /// above 0 to 0; a timer at 0 is never counted down.
    DisarmTheFlightDefeatTimer,
    /// In state 0xC5 the game asks for this copy's defeat flow at a hit with health 0 or below.
    KeepTheHealthAtOne,
}

pub fn partner_body_guard_act(body: PartnerBodyReading) -> PartnerBodyGuardAct {
    match body.state {
        BODY_FLIGHT_DEFEAT_STATE
            if body.flight_defeat_timer > FLIGHT_DEFEAT_TIMER_THAT_NEVER_ASKS =>
        {
            PartnerBodyGuardAct::DisarmTheFlightDefeatTimer
        }
        BODY_STATE_THAT_ASKS_FOR_A_DEFEAT_AT_NO_HEALTH
            if body.health < LOWEST_HEALTH_KEPT_ON_THE_PARTNER_BODY =>
        {
            PartnerBodyGuardAct::KeepTheHealthAtOne
        }
        _ => PartnerBodyGuardAct::Nothing,
    }
}

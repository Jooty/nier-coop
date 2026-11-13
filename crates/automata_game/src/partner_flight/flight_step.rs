use crate::partner_flight::FlightMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlightCall {
    /// `startPhaseEventFlightUnit` on the body.
    Board,
    /// The mode setter of this mode on the body, with its "now" argument set.
    SetMode(FlightMode),
    /// `endPhaseEventFlightUnit` on the body.
    GetOff,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FlightStep {
    pub call: Option<FlightCall>,
    /// The body's unit is to be corrected toward the partner's reported unit.
    pub place_unit: bool,
    /// The flight stick to feed into the body's controller block this tick.
    pub feed_stick: Option<[f32; 2]>,
    /// The body rides: its own position is corrected by plain writes, never the game's teleport.
    pub body_rides: bool,
    pub logs: Vec<String>,
}

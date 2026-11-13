use crate::partner_flight::{FlightMode, FlightSteering};

/// The flight fields of the partner body on this game, read this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyFlight {
    pub riding: bool,
    /// The body's mode word read as a mode; it means something only while it rides.
    pub mode: FlightMode,
    pub state: i32,
    pub alive: bool,
    pub shooting_stage: bool,
    /// The body's unit handle resolves to a live flight unit.
    pub unit_found: bool,
    pub steering: FlightSteering,
}

impl BodyFlight {
    /// The riding flag, the shooting stage flag or a unit handle that resolves.
    pub fn is_in_a_unit(&self) -> bool {
        self.riding || self.shooting_stage || self.unit_found
    }
}

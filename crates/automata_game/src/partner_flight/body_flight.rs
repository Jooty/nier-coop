use crate::partner_flight::FlightMode;

/// The flight fields of the partner body on this game, read this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BodyFlight {
    pub riding: bool,
    /// The body's mode word read as a mode; it means something only while it rides.
    pub mode: FlightMode,
    pub state: i32,
    pub alive: bool,
}

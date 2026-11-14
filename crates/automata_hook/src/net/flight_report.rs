use std::sync::Mutex;

use automata_game::partner_flight::{FlightKind, FlightMode, FlightReport};

static LOCAL_FLIGHT_REPORT: Mutex<FlightReport> = Mutex::new(FlightReport {
    riding: false,
    kind: FlightKind::NotInAUnit,
    mode: FlightMode::None,
    unit: None,
    stick: None,
});

/// What every state message of this game says about the hero's flight unit from now on.
pub fn set_local_flight_report(report: FlightReport) {
    if let Ok(mut held) = LOCAL_FLIGHT_REPORT.lock() {
        *held = report;
    }
}

pub fn local_flight_report() -> FlightReport {
    LOCAL_FLIGHT_REPORT
        .lock()
        .map(|held| *held)
        .unwrap_or_default()
}

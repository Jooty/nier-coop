use core::f32::consts::PI;

use serde::{Deserialize, Serialize};

use crate::partner_flight::{
    FLIGHT_STICK_AXIS_SENT_AT_MOST, FLIGHT_UNIT_POSITION_SENT_AT_MOST, FlightKind, FlightMode,
    UnitPose,
};

/// What a hero's game says about the hero's flight unit, sent on every state message.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct FlightReport {
    /// The rider's own riding flag, as read.
    pub riding: bool,
    pub kind: FlightKind,
    pub mode: FlightMode,
    /// The unit found through the rider's unit handle; none out of a unit or when not found.
    pub unit: Option<UnitPose>,
    /// The stick vector the rider's flight movement reads, before the camera turns it; none out
    /// of a unit.
    pub stick: Option<[f32; 2]>,
}

impl FlightReport {
    pub fn is_in_a_unit(&self) -> bool {
        self.kind.is_in_a_unit()
    }

    /// A report `flight_report_of` can make: bounded numbers, and a mode and a stick exactly when
    /// in a unit.
    pub fn is_valid(&self) -> bool {
        let in_a_unit = self.is_in_a_unit();
        let mode_fits = (self.mode == FlightMode::None) != in_a_unit;
        let stick_fits = match self.stick {
            None => !in_a_unit,
            Some(stick) => {
                in_a_unit
                    && stick.iter().all(|axis| {
                        axis.is_finite() && axis.abs() <= FLIGHT_STICK_AXIS_SENT_AT_MOST
                    })
            }
        };
        let unit_fits = self.unit.is_none_or(|unit| {
            in_a_unit
                && unit.yaw.is_finite()
                && unit.yaw.abs() <= PI
                && unit
                    .position
                    .iter()
                    .all(|axis| axis.is_finite() && axis.abs() <= FLIGHT_UNIT_POSITION_SENT_AT_MOST)
        });
        mode_fits && stick_fits && unit_fits
    }
}

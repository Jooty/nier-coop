use serde::{Deserialize, Serialize};

use crate::partner::facing_error;
use crate::partner_flight::{
    FLIGHT_STICK_AXIS_SENT_AT_MOST, FLIGHT_UNIT_POSITION_SENT_AT_MOST, FlightKind, FlightMode,
    FlightReport,
};

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct UnitPose {
    pub position: [f32; 3],
    pub yaw: f32,
}

/// The flight fields of a rider as read from its body.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FlightFields {
    pub riding: i32,
    pub shooting_stage: i32,
    pub mode_word: i32,
    pub unit: Option<UnitPose>,
    pub stick: [f32; 2],
}

pub fn flight_report_of(fields: &FlightFields) -> FlightReport {
    let kind = if fields.shooting_stage != 0 {
        FlightKind::ShootingStage
    } else if fields.riding != 0 {
        FlightKind::FreeFlight
    } else {
        FlightKind::NotInAUnit
    };
    if !kind.is_in_a_unit() {
        return FlightReport {
            riding: false,
            ..FlightReport::default()
        };
    }
    let unit = fields
        .unit
        .filter(|unit| {
            unit.yaw.is_finite()
                && unit
                    .position
                    .iter()
                    .all(|axis| axis.is_finite() && axis.abs() <= FLIGHT_UNIT_POSITION_SENT_AT_MOST)
        })
        .map(|unit| UnitPose {
            yaw: facing_error(0.0, unit.yaw),
            ..unit
        });
    FlightReport {
        riding: fields.riding != 0,
        kind,
        mode: FlightMode::of_mode_word(fields.mode_word),
        unit,
        stick: Some(fields.stick.map(|axis| {
            if axis.is_finite() {
                axis.clamp(
                    -FLIGHT_STICK_AXIS_SENT_AT_MOST,
                    FLIGHT_STICK_AXIS_SENT_AT_MOST,
                )
            } else {
                0.0
            }
        })),
    }
}

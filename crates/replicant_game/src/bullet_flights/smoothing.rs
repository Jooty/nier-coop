use replicant_proto::BULLET_TRANSFORM_FLOATS;

use crate::bullet_flights::{BULLET_RENDER_DELAY_MS, MAX_EXTRAPOLATION_MS};

const MILLIS_PER_SECOND: f32 = 1000.0;
const TRANSLATION_COLUMN: usize = 3;
const TRANSFORM_ROW_FLOATS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FlightSample {
    pub transform: [f32; BULLET_TRANSFORM_FLOATS],
    pub velocity: [f32; 3],
    pub received_ms: u64,
}

pub fn smoothed_transform(
    previous: Option<&FlightSample>,
    latest: &FlightSample,
    now_ms: u64,
) -> [f32; BULLET_TRANSFORM_FLOATS] {
    let Some(previous) = previous else {
        return extrapolated(latest, now_ms.saturating_sub(latest.received_ms));
    };
    let render_ms = now_ms.saturating_sub(BULLET_RENDER_DELAY_MS);
    if render_ms >= latest.received_ms {
        return extrapolated(latest, render_ms - latest.received_ms);
    }
    let span_ms = latest.received_ms.saturating_sub(previous.received_ms);
    if span_ms == 0 || render_ms <= previous.received_ms {
        return previous.transform;
    }
    let fraction = (render_ms - previous.received_ms) as f32 / span_ms as f32;
    let mut transform = latest.transform;
    for axis in 0..3 {
        let index = axis * TRANSFORM_ROW_FLOATS + TRANSLATION_COLUMN;
        transform[index] = previous.transform[index]
            + (latest.transform[index] - previous.transform[index]) * fraction;
    }
    transform
}

fn extrapolated(sample: &FlightSample, age_ms: u64) -> [f32; BULLET_TRANSFORM_FLOATS] {
    let seconds = age_ms.min(MAX_EXTRAPOLATION_MS) as f32 / MILLIS_PER_SECOND;
    let mut transform = sample.transform;
    for (axis, speed) in sample.velocity.into_iter().enumerate() {
        transform[axis * TRANSFORM_ROW_FLOATS + TRANSLATION_COLUMN] += speed * seconds;
    }
    transform
}

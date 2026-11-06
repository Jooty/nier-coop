use std::sync::atomic::{AtomicU32, Ordering};

use rc_game::zone_rules::{DEFAULT_ZONE_RADIUS_UNITS, ZoneRadius};

static ZONE_RADIUS_BITS: AtomicU32 = AtomicU32::new(DEFAULT_ZONE_RADIUS_UNITS.to_bits());

pub fn zone_radius() -> ZoneRadius {
    ZoneRadius::new(f32::from_bits(ZONE_RADIUS_BITS.load(Ordering::Relaxed)))
}

pub fn set_zone_radius(units: f32) {
    ZONE_RADIUS_BITS.store(ZoneRadius::new(units).units().to_bits(), Ordering::Relaxed);
}

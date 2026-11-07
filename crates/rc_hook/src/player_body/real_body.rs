use std::sync::{Mutex, PoisonError};

use rc_game::puppet::body_for_seed;
use rc_proto::PartnerBody;

static REAL_BODY: Mutex<Option<RealBody>> = Mutex::new(None);
static PHASE_LOADS: Mutex<u32> = Mutex::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RealBody {
    pub body: PartnerBody,
    pub phase_loads: u32,
}

pub fn real_body() -> Option<RealBody> {
    *REAL_BODY.lock().unwrap_or_else(PoisonError::into_inner)
}

pub(super) fn note_seed_in_use(seed_name: &str) -> Option<RealBody> {
    let mut loads = PHASE_LOADS.lock().unwrap_or_else(PoisonError::into_inner);
    *loads = loads.wrapping_add(1);
    let noted = body_for_seed(seed_name).map(|body| RealBody {
        body,
        phase_loads: *loads,
    });
    *REAL_BODY.lock().unwrap_or_else(PoisonError::into_inner) = noted;
    noted
}

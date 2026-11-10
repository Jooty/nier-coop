use core::ffi::c_void;

use replicant_game::offsets::{
    ENVIRONMENT_MANAGER_RVA, ENVIRONMENT_SET_AREA_FUNCTION_RVA, WIND_MANAGER_RVA,
    WIND_SET_AREA_FUNCTION_RVA,
};

use crate::build_gate::VerifiedBuild;

type EnvironmentSetArea = unsafe extern "system" fn(*mut c_void, i32, f32);
type WindSetArea = unsafe extern "system" fn(*mut c_void, i32, f32);

#[derive(Debug, Clone, Copy)]
pub(super) struct SkyCalls {
    module_base: u64,
}

impl SkyCalls {
    pub(super) fn new(build: &VerifiedBuild) -> Self {
        Self {
            module_base: build.module_base(),
        }
    }

    pub(super) fn set_environment_area(&self, area: i32, blend_seconds: f32) {
        let function = (self.module_base + u64::from(ENVIRONMENT_SET_AREA_FUNCTION_RVA)) as usize;
        let manager = (self.module_base + u64::from(ENVIRONMENT_MANAGER_RVA)) as usize;
        #[expect(
            unsafe_code,
            reason = "calls the environment manager's area setter at its verified address"
        )]
        // SAFETY: the build gate passed, so the RVA starts the setter the scripts' area natives
        // end in (manager, area, blend seconds); it rejects a missing table or area >= 0x80
        // itself. This runs on the game thread inside the script manager tick.
        unsafe {
            core::mem::transmute::<usize, EnvironmentSetArea>(function)(
                manager as *mut c_void,
                area,
                blend_seconds,
            );
        }
    }

    pub(super) fn set_wind_area(&self, area: i32, blend_seconds: f32) {
        let function = (self.module_base + u64::from(WIND_SET_AREA_FUNCTION_RVA)) as usize;
        let manager = (self.module_base + u64::from(WIND_MANAGER_RVA)) as usize;
        #[expect(
            unsafe_code,
            reason = "calls the wind manager's area setter at its verified address"
        )]
        // SAFETY: the build gate passed, so the RVA starts the wind setter (manager, area, blend
        // seconds) that the _SetActiveArea native calls. This runs on the game thread inside the
        // script manager tick.
        unsafe {
            core::mem::transmute::<usize, WindSetArea>(function)(
                manager as *mut c_void,
                area,
                blend_seconds,
            );
        }
    }
}

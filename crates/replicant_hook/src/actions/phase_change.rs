use core::ffi::c_void;
use std::fmt;

use replicant_game::offsets::{
    CLEAR_PHASE_LOADING_FLAG_FUNCTION_RVA, GAME_SINGLETON_FUNCTION_RVA, PHASE_NAME_BUFFER_LEN,
    PLAYER_STATE_OF_SINGLETON_FUNCTION_RVA, PLAYER_STATE_PHASE_CHANGE_FLAG_OFFSET,
    REQUEST_PHASE_CHANGE_FUNCTION_RVA,
};
use replicant_game::{GameError, GameMemory, MemoryError, PhaseEntry, Replicant};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;

type RequestPhaseChange = unsafe extern "system" fn(*mut c_void, *const u8, i32);
type ClearPhaseLoadingFlag = unsafe extern "system" fn(u8);
type GameSingleton = unsafe extern "system" fn() -> *mut c_void;
type PlayerStateOfSingleton = unsafe extern "system" fn(*mut c_void) -> *mut c_void;

#[derive(Debug)]
pub(crate) enum PhaseChangeError {
    Game(GameError),
    MapManagerInactive,
    NameDoesNotFit,
}

impl From<GameError> for PhaseChangeError {
    fn from(error: GameError) -> Self {
        Self::Game(error)
    }
}

impl From<MemoryError> for PhaseChangeError {
    fn from(error: MemoryError) -> Self {
        Self::Game(GameError::Memory(error))
    }
}

impl fmt::Display for PhaseChangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Game(error) => fmt::Display::fmt(error, formatter),
            Self::MapManagerInactive => write!(formatter, "the map manager has no active context"),
            Self::NameDoesNotFit => write!(formatter, "phase name does not fit the game buffer"),
        }
    }
}

pub(crate) fn change_phase(
    build: &VerifiedBuild,
    phase: &PhaseEntry,
    point: i32,
) -> Result<(), PhaseChangeError> {
    let mut game = Replicant::new(InProcessMemory::new(build));
    if !game.map_manager_is_active()? {
        return Err(PhaseChangeError::MapManagerInactive);
    }
    if phase.name.len() >= PHASE_NAME_BUFFER_LEN {
        return Err(PhaseChangeError::NameDoesNotFit);
    }
    let mut name = [0_u8; PHASE_NAME_BUFFER_LEN];
    name[..phase.name.len()].copy_from_slice(phase.name.as_bytes());
    let map_manager = game.map_manager_address() as usize as *mut c_void;
    let module_base = build.module_base();
    #[expect(
        unsafe_code,
        reason = "calls the game's own phase change functions at their verified addresses"
    )]
    // SAFETY: the build gate passed, so each RVA starts the function Ghidra named there with the
    // signature declared above. The map manager context was checked non-null just before, and
    // this runs on the game thread inside the script manager tick, where `_ChangeMap` itself runs.
    let player_state = unsafe {
        let request: RequestPhaseChange = core::mem::transmute::<usize, RequestPhaseChange>(
            (module_base + u64::from(REQUEST_PHASE_CHANGE_FUNCTION_RVA)) as usize,
        );
        let clear_flag: ClearPhaseLoadingFlag = core::mem::transmute::<usize, ClearPhaseLoadingFlag>(
            (module_base + u64::from(CLEAR_PHASE_LOADING_FLAG_FUNCTION_RVA)) as usize,
        );
        let singleton: GameSingleton = core::mem::transmute::<usize, GameSingleton>(
            (module_base + u64::from(GAME_SINGLETON_FUNCTION_RVA)) as usize,
        );
        let player_state_of: PlayerStateOfSingleton =
            core::mem::transmute::<usize, PlayerStateOfSingleton>(
                (module_base + u64::from(PLAYER_STATE_OF_SINGLETON_FUNCTION_RVA)) as usize,
            );
        request(map_manager, name.as_ptr(), point);
        clear_flag(0);
        player_state_of(singleton())
    };
    if !player_state.is_null() {
        let flag_address = player_state as usize as u64 + PLAYER_STATE_PHASE_CHANGE_FLAG_OFFSET;
        game.memory_mut().write_i32(flag_address, 0)?;
    }
    Ok(())
}

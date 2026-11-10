use automata_game::address::{
    CHAPTER_MARKER_FUNCTION, CLEAR_FLAG_BIT_FUNCTION, CURRENT_HERO_FUNCTION, NAME_HASH_FUNCTION,
    NAME_INTERN_FUNCTION, NAME_INTERN_TABLE, NAME_RELEASE_FUNCTION, PHASE_START_FUNCTION,
    SCENE_STATE_SET_FLAG_FUNCTION, SCENE_STATE_SYSTEM_OBJECT,
};

use crate::game_calls::GameCalls;

type NameHash = unsafe extern "system" fn(*const u8, usize) -> u64;
type NameIntern = unsafe extern "system" fn(usize, u64, *const u8, usize) -> u64;
type SetSceneFlag = unsafe extern "system" fn(usize, *const u64);
type NameRelease = unsafe extern "system" fn(*mut u64);
type NoArgumentCall = unsafe extern "system" fn();
type ClearBit = unsafe extern "system" fn(usize);
type PhaseStart = unsafe extern "system" fn(usize, *const u8, usize) -> usize;
type CurrentHero = unsafe extern "system" fn() -> usize;

impl GameCalls {
    #[expect(
        unsafe_code,
        reason = "calls the game's name hash, name intern and name release functions"
    )]
    pub(super) fn with_interned_name<T>(&self, name: &str, use_name: impl FnOnce(&u64) -> T) -> T {
        let mut text = name.as_bytes().to_vec();
        text.push(0);
        // SAFETY: the addresses are the verified build's functions (a guess for the signatures
        // until run); `text` is a NUL-terminated buffer that outlives the calls, and the interned
        // value lives on this stack frame while the caller and the release function use it.
        unsafe {
            let hash = core::mem::transmute::<usize, NameHash>(self.address_of(NAME_HASH_FUNCTION));
            let intern =
                core::mem::transmute::<usize, NameIntern>(self.address_of(NAME_INTERN_FUNCTION));
            let release =
                core::mem::transmute::<usize, NameRelease>(self.address_of(NAME_RELEASE_FUNCTION));
            let hashed = hash(text.as_ptr(), name.len());
            let mut interned = intern(
                self.address_of(NAME_INTERN_TABLE),
                hashed,
                text.as_ptr(),
                name.len(),
            );
            let result = use_name(&interned);
            // The game gives the interned name back after every use; it is null safe.
            release(&mut interned);
            result
        }
    }

    #[expect(unsafe_code, reason = "calls the game's flag set function")]
    pub fn set_scene_flag(&self, name: &str) {
        self.with_interned_name(name, |interned| {
            // SAFETY: the address is the flag set function of the verified build (a guess for
            // the signature until run); the interned name outlives the call.
            unsafe {
                core::mem::transmute::<usize, SetSceneFlag>(
                    self.address_of(SCENE_STATE_SET_FLAG_FUNCTION),
                )(self.address_of(SCENE_STATE_SYSTEM_OBJECT), interned);
            }
        });
    }

    #[expect(unsafe_code, reason = "calls the game's chapter marker function")]
    pub fn run_marker_call(&self) {
        // SAFETY: the address is the marker function of the verified build; it takes no arguments.
        unsafe {
            core::mem::transmute::<usize, NoArgumentCall>(self.address_of(CHAPTER_MARKER_FUNCTION))(
            );
        }
    }

    #[expect(unsafe_code, reason = "calls the game's clear-flag-bit function")]
    pub fn clear_flag_bit(&self, bit: u32) {
        // SAFETY: the address is the ClearBit function of the verified build; it takes the bit index.
        unsafe {
            core::mem::transmute::<usize, ClearBit>(self.address_of(CLEAR_FLAG_BIT_FUNCTION))(
                bit as usize,
            );
        }
    }

    #[expect(unsafe_code, reason = "calls the game's chapter phase start function")]
    pub fn phase_start(&self, phase_id: i32, name_with_nul: &[u8], chapter_filter: i32) -> bool {
        // SAFETY: the address is the phase start function of the verified build; the name is a
        // NUL-terminated buffer owned by the caller for the life of the process.
        let result = unsafe {
            core::mem::transmute::<usize, PhaseStart>(self.address_of(PHASE_START_FUNCTION))(
                phase_id as u32 as usize,
                name_with_nul.as_ptr(),
                chapter_filter as u32 as usize,
            )
        };
        result & 0xFF != 0
    }

    #[expect(unsafe_code, reason = "calls the game's current hero getter")]
    pub fn current_hero_present(&self) -> bool {
        // SAFETY: the address is the hero getter of the verified build; it takes no arguments.
        unsafe {
            core::mem::transmute::<usize, CurrentHero>(self.address_of(CURRENT_HERO_FUNCTION))()
                != 0
        }
    }
}

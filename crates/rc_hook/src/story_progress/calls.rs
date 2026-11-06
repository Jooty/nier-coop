use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::dialogue::GAME_STATE_RVA;
use rc_game::offsets::ENTITY_RVA;
use rc_game::story_progress::{
    GAME_COUNTER_SET_FUNCTION_RVA, GAME_FLAG_CLEAR_FUNCTION_RVA, GAME_FLAG_SET_FUNCTION_RVA,
    KEY_FRAGMENTS_BLOCK_OFFSET, LAST_LOADED_SLOT_RVA, PLAYER_NAME_BYTES,
    SNOW_COUNTER_SET_FUNCTION_RVA, SNOW_FLAG_CLEAR_FUNCTION_RVA, SNOW_FLAG_SET_FUNCTION_RVA,
    STORY_BLOCK_READ_BYTES, SlotStory, StoryArrays, slot_player_name,
};
use rc_proto::{StoryArray, StoryChange};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;

type FlagSetter = unsafe extern "system" fn(*mut c_void, u32);
type CounterSetter = unsafe extern "system" fn(*mut c_void, u32, u8);

pub(super) fn read_live_block(build: &VerifiedBuild) -> Option<Vec<u8>> {
    let memory = InProcessMemory::new(build);
    let mut block = vec![0_u8; STORY_BLOCK_READ_BYTES];
    memory
        .read(build.module_base() + u64::from(ENTITY_RVA), &mut block)
        .ok()?;
    Some(block)
}

pub(crate) fn read_live_arrays(build: &VerifiedBuild) -> Option<StoryArrays> {
    StoryArrays::from_block(&read_live_block(build)?)
}

pub(super) fn read_live_story(
    build: &VerifiedBuild,
) -> Option<(SlotStory, [u8; PLAYER_NAME_BYTES])> {
    let block = read_live_block(build)?;
    let name = slot_player_name(&block)?.try_into().ok()?;
    Some((SlotStory::read(&block)?, name))
}

pub(super) fn read_loaded_slot(build: &VerifiedBuild) -> Option<u32> {
    let memory = InProcessMemory::new(build);
    let mut bytes = [0_u8; 4];
    memory
        .read(
            build.module_base() + u64::from(LAST_LOADED_SLOT_RVA),
            &mut bytes,
        )
        .ok()?;
    Some(u32::from_le_bytes(bytes))
}

fn write_key_fragment(build: &VerifiedBuild, id: u8, value: u8) {
    let mut memory = InProcessMemory::new(build);
    let address = build.module_base()
        + u64::from(ENTITY_RVA)
        + KEY_FRAGMENTS_BLOCK_OFFSET as u64
        + u64::from(id);
    if memory.write(address, &[value]).is_err() {
        log_line(&format!("story: could not store key fragment slot {id}"));
    }
}

#[expect(
    unsafe_code,
    reason = "calls the game's own flag and counter setters by their addresses"
)]
pub(crate) fn apply_change(build: &VerifiedBuild, change: StoryChange) {
    let base = build.module_base();
    let owner = (base + u64::from(GAME_STATE_RVA)) as *mut c_void;
    match change {
        StoryChange::Flag { array, id, on } => {
            let rva = match (array, on) {
                (StoryArray::GameFlags, true) => GAME_FLAG_SET_FUNCTION_RVA,
                (StoryArray::GameFlags, false) => GAME_FLAG_CLEAR_FUNCTION_RVA,
                (StoryArray::SnowFlags, true) => SNOW_FLAG_SET_FUNCTION_RVA,
                (StoryArray::SnowFlags, false) => SNOW_FLAG_CLEAR_FUNCTION_RVA,
                _ => return,
            };
            let function = (base + u64::from(rva)) as *const ();
            // SAFETY: the RVA starts one of the game's flag setters (owner, id) in the verified
            // build; each ignores ids of 0x800 and above and only reads the owner pointer; this
            // runs on the game thread before the script tick.
            unsafe {
                core::mem::transmute::<*const (), FlagSetter>(function)(owner, u32::from(id));
            }
        }
        StoryChange::Counter {
            array: StoryArray::KeyFragments,
            id,
            value,
        } => write_key_fragment(build, id, value),
        StoryChange::Counter { array, id, value } => {
            let rva = match array {
                StoryArray::GameCounters => GAME_COUNTER_SET_FUNCTION_RVA,
                StoryArray::SnowCounters => SNOW_COUNTER_SET_FUNCTION_RVA,
                _ => return,
            };
            let function = (base + u64::from(rva)) as *const ();
            // SAFETY: the RVA starts one of the game's counter stores (owner, id, value) in the
            // verified build; each ignores ids of 0x100 and above and writes one byte.
            unsafe {
                core::mem::transmute::<*const (), CounterSetter>(function)(
                    owner,
                    u32::from(id),
                    value,
                );
            }
        }
    }
}

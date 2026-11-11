use core::ffi::c_void;

use replicant_game::GameMemory;
use replicant_game::build::SUPPORTED_PE_IDENTITY;
use replicant_game::offsets::ACTOR_FORCE_KILL_VTABLE_OFFSET;

use crate::downed::kill_guard::OwnKill;
use crate::memory::InProcessMemory;

type ForceKill = unsafe extern "system" fn(*mut c_void);

pub(super) fn force_kill(memory: &InProcessMemory, module_base: u64, actor: u64) -> bool {
    let image_end = module_base + u64::from(SUPPORTED_PE_IDENTITY.size_of_image);
    let in_image = |address: u64| (module_base..image_end).contains(&address);
    let Ok(vtable) = memory.read_u64(actor) else {
        return false;
    };
    if !in_image(vtable) {
        return false;
    }
    let Ok(function) = memory.read_u64(vtable + ACTOR_FORCE_KILL_VTABLE_OFFSET) else {
        return false;
    };
    if !in_image(function) {
        return false;
    }
    #[expect(
        unsafe_code,
        reason = "calls ActorPlayable::ForceKill through the player's vtable"
    )]
    // SAFETY: the actor is the live playable actor read this tick, slot 0x580 of its vtable is
    // ForceKill (this, void) in the verified build and lies inside the image, and this runs on
    // the game thread inside the script manager tick.
    unsafe {
        let _own_kill = OwnKill::begin();
        core::mem::transmute::<usize, ForceKill>(function as usize)(actor as usize as *mut c_void);
    }
    true
}

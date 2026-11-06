use core::ffi::c_void;

use minhook::MinHook;
use rc_game::build::SUPPORTED_SIZE_OF_IMAGE;

use crate::build_gate::VerifiedBuild;
use crate::detour::DetourError;

#[derive(Debug)]
pub struct Detour {
    target: *mut c_void,
    trampoline: *mut c_void,
}

impl Detour {
    /// # Safety
    /// `rva` must start a function in the verified build; `replacement` must match its signature.
    #[expect(
        unsafe_code,
        reason = "hooking a function by address is inherently unchecked"
    )]
    pub unsafe fn create(
        build: &VerifiedBuild,
        rva: u32,
        replacement: *mut c_void,
    ) -> Result<Self, DetourError> {
        if rva >= SUPPORTED_SIZE_OF_IMAGE {
            return Err(DetourError::RvaOutsideImage { rva });
        }
        let target = (build.module_base() + u64::from(rva)) as usize;
        // SAFETY: the caller upholds the function and signature contract above.
        unsafe { Self::create_at_address(target, replacement) }
    }

    /// # Safety
    /// `address` must start a function in this process; `replacement` must match its signature.
    #[expect(
        unsafe_code,
        reason = "hooking a function by address is inherently unchecked"
    )]
    pub unsafe fn create_at_address(
        address: usize,
        replacement: *mut c_void,
    ) -> Result<Self, DetourError> {
        let target = address as *mut c_void;
        // SAFETY: the caller upholds the function and signature contract above.
        let trampoline = unsafe { MinHook::create_hook(target, replacement) }?;
        Ok(Self { target, trampoline })
    }

    #[expect(
        unsafe_code,
        reason = "enabling a hook rewrites code in the running game"
    )]
    pub fn enable(&self) -> Result<(), DetourError> {
        // SAFETY: the hook for `target` was created by `create` and is enabled while it exists.
        unsafe { MinHook::enable_hook(self.target) }?;
        Ok(())
    }

    /// # Safety
    /// Same contract as [`Detour::create`].
    #[expect(
        unsafe_code,
        reason = "hooking a function by address is inherently unchecked"
    )]
    pub unsafe fn install(
        build: &VerifiedBuild,
        rva: u32,
        replacement: *mut c_void,
    ) -> Result<Self, DetourError> {
        // SAFETY: forwarded from this function's own contract.
        let detour = unsafe { Self::create(build, rva, replacement) }?;
        detour.enable()?;
        Ok(detour)
    }

    pub fn trampoline(&self) -> *mut c_void {
        self.trampoline
    }

    #[expect(
        unsafe_code,
        reason = "removing a hook rewrites code in the running game"
    )]
    pub fn uninstall(self) -> Result<(), DetourError> {
        // SAFETY: `target` was hooked by `install` and is hooked only once.
        unsafe {
            MinHook::disable_hook(self.target)?;
            MinHook::remove_hook(self.target)?;
        }
        Ok(())
    }
}

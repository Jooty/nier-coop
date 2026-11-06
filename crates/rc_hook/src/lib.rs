#[cfg(windows)]
mod build_gate;
#[cfg(windows)]
mod detour;
#[cfg(windows)]
mod diagnostics;
#[cfg(windows)]
mod entry;
#[cfg(windows)]
mod memory;
#[cfg(windows)]
mod overlay;
#[cfg(windows)]
mod proxy;

#[cfg(windows)]
pub use build_gate::{GateError, VerifiedBuild};
#[cfg(windows)]
pub use detour::{Detour, DetourError};
#[cfg(windows)]
pub use memory::InProcessMemory;

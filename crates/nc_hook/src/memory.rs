mod error;
mod game_memory;
#[cfg(windows)]
mod in_process;
mod page_protection;

pub use error::MemoryError;
pub use game_memory::GameMemory;
#[cfg(windows)]
pub use in_process::InProcessMemory;
pub use page_protection::{WriteAccess, is_executable, write_access};

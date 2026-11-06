mod error;
mod game_memory;
mod page_protection;

pub use error::MemoryError;
pub use game_memory::GameMemory;
pub use page_protection::{WriteAccess, is_executable, write_access};

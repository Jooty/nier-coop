mod kind;
mod lookup;
mod table;

pub use kind::StoryCallKind;
pub use lookup::{kind_in_table, name_hash, story_call_kind, story_call_kind_by_name};
pub use table::STORY_CALL_TABLE;

pub const LIBRARY_PACK_HASH: u64 = 0x7588_BE80_E701_6100;

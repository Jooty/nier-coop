mod game_address;
mod known_addresses;

pub use game_address::GameAddress;
pub use known_addresses::{
    DIRECT_INPUT8_CREATE_IMPORT_SLOT, ENTRY_POINT, FRAME_END_FUNCTION, INPUT_UPDATE_ALL_FUNCTION,
    PAD_INPUT_RECORDS,
};

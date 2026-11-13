mod campaign_load;
mod chapter_table_read;
mod driver;
mod driver_state;
mod link_reading;

pub use driver::{install, status_line, tick};

const SCENE_NAME_READ_LIMIT: usize = 0x20;
const NOT_READ_SENTINEL: i32 = -99;

//! Which UI scenes the game's UI manager has open: the pause menu signal for "in menu" and no-pause.

mod open_scenes;

pub use open_scenes::{OpenUiScenes, read_open_ui_scenes};

pub const UI_MANAGER_RVA: u32 = 0x17B_90D0;
pub const UI_MANAGER_SCENE_LIST_OFFSET: u64 = 0x60;
pub const UI_MANAGER_FIRST_SCENE_LINK_OFFSET: u64 = 0x68;
pub const UI_LINK_NEXT_OFFSET: u64 = 0x8;
pub const UI_SCENE_ID_OFFSET: u64 = 0x18;
pub const UI_SCENE_SEQUENCE_LIST_OFFSET: u64 = 0x30;
pub const UI_SCENE_FIRST_SEQUENCE_LINK_OFFSET: u64 = 0x38;
pub const SHORT_SYSTEM_MENU_OPEN_FLAG_OFFSET: u64 = 0x214;
pub const UI_SCENE_WALK_LIMIT: usize = 32;
pub const SHORT_SYSTEM_MENU_SCENE_ID: u8 = 1;
pub const PAUSE_MENU_SCENE_ID: u8 = 2;
pub const LARGE_MAP_SCENE_ID: u8 = 10;
pub const OPTION_MENU_SCENE_ID: u8 = 13;
pub const QUICK_MAGIC_EDIT_SCENE_ID: u8 = 19;
pub const PAUSE_MENU_SCENE_IDS: [u8; 4] = [
    PAUSE_MENU_SCENE_ID,
    LARGE_MAP_SCENE_ID,
    OPTION_MENU_SCENE_ID,
    QUICK_MAGIC_EDIT_SCENE_ID,
];
pub const PAUSE_MENU_CLOSE_MAX_REQUESTS_PER_SCENE_START: u8 = 2;

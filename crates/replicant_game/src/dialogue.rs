//! The game's talk window: which talk and line a player is reading, and mirroring the partner's.
mod choice_follow;
mod choice_hover;
mod mirror;
mod talk_cursor;
mod talker_name;
mod tracker;
mod watcher_hold;

pub use choice_follow::{
    ChoiceFollowInput, ChoiceFollower, ChoiceStep, FollowClock, open_prompt_kind,
};
pub use choice_hover::{
    ChoiceHoverTracker, ChoicePrompt, FollowedPromptFrame, OwnChoiceMarker, PartnerChoiceHover,
    PromptPick, followed_prompt_frame, host_answer_fits, marker_move, prompt_pick,
    read_choice_prompt, write_highlighted_option,
};
pub use mirror::{
    MirrorAction, MirrorInput, PartnerTalkMemory, PartnerTalkMirror, TalkLeader,
    leader_for_own_talk, talk_tags_run_here, within_talk_range,
};
pub use talk_cursor::{TalkCursor, find_talk_id, read_talk_cursor};
pub use talker_name::{replace_player_name_tag, speaker_name_to_show};
pub use tracker::DialogueTracker;
pub use watcher_hold::{WatcherHold, WatcherHoldAction, WatcherHoldInput, own_talk_started};

pub const TALK_WINDOW_RVA: u32 = 0x4C5_6970;
pub const TALK_WINDOW_OPEN_RVA: u32 = 0x4D_FF00;
pub const TALK_WINDOW_IS_BUSY_RVA: u32 = 0x4D_FAE0;
pub const TALK_WINDOW_UPDATE_RVA: u32 = 0x4E_0AF0;
pub const TALK_WINDOW_CLOSE_RVA: u32 = 0x4E_0A50;
pub const TALK_WINDOW_STATE_TABLE_RVA: u32 = 0x12F_3898;
pub const TALK_WINDOW_STATE_COUNT: i32 = 4;
pub const TALK_PLAYER_NEXT_LINE_RVA: u32 = 0x4E_3530;
pub const TALK_WINDOW_PLAYER_OFFSET: u64 = 0x108;
pub const TALK_WINDOW_CURRENT_TALK_OFFSET: u64 = 0x46_3040;
pub const TALK_WINDOW_CURRENT_LINE_OFFSET: u64 = 0x46_3048;
pub const TALK_WINDOW_MASK_BEFORE_OPEN_OFFSET: u64 = 0x4A_4170;
pub const TALK_WINDOW_TALK_TABLE_OFFSET: u64 = 0x1C_2B18;
pub const TALK_TABLE_SLOTS: u32 = 0x4_0000;
pub const TALK_TABLE_SCAN_CHUNK_SLOTS: u32 = 0x200;
pub const TALK_UPDATE_INPUT_BYTES: usize = 0x48;
pub const TALK_UPDATE_INPUT_ENABLED_OFFSET: usize = 0x42;
pub const TALK_UPDATE_INPUT_BUTTONS_START_OFFSET: usize = 0x20;
pub const TALK_UPDATE_INPUT_POINTER_ACTIVE_OFFSET: usize = 0x44;
pub const TALK_UPDATE_INPUT_BUTTON_PRESSED_BIT: u8 = 1;
pub const TALK_UPDATE_INPUT_VERTICAL_PREVIOUS_OFFSETS: [usize; 4] = [0x20, 0x24, 0x3B, 0x3F];
pub const TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS: [usize; 4] = [0x21, 0x25, 0x3C, 0x40];
pub const TALK_UPDATE_INPUT_HORIZONTAL_PREVIOUS_OFFSETS: [usize; 4] = [0x22, 0x26, 0x39, 0x3D];
pub const TALK_UPDATE_INPUT_HORIZONTAL_NEXT_OFFSETS: [usize; 4] = [0x23, 0x27, 0x3A, 0x3E];
pub const TALK_UPDATE_INPUT_ENTER_LIST_OFFSET: usize = TALK_UPDATE_INPUT_VERTICAL_NEXT_OFFSETS[0];
pub const TALK_PLAYER_PROMPT_KIND_OFFSET: u64 = 0x46_3068;
pub const TALK_PLAYER_CHOICE_SELECTOR_OFFSET: u64 = 0x46_2A28;
pub const CHOICE_SELECTOR_OPTION_COUNT_OFFSET: u64 = 0x20;
pub const CHOICE_SELECTOR_AXIS_OFFSET: u64 = 0x24;
pub const CHOICE_SELECTOR_AXIS_VERTICAL: i32 = 0;
pub const CHOICE_SELECTOR_AXIS_HORIZONTAL: i32 = 1;
pub const CHOICE_SELECTOR_HIGHLIGHTED_INDEX_OFFSET: u64 = 0x28;
pub const CHOICE_SELECTOR_CANCEL_INDEX_OFFSET: u64 = 0x60;
pub const TALK_STATE_PROMPT_CURSOR: i32 = 6;
pub const TALK_STATE_PROMPT_TALK_LIST: i32 = 7;
pub const TALK_STATE_PROMPT_MENU_LIST: i32 = 9;
pub const TALK_PROMPT_KIND_TALK_LIST: i32 = 3;
pub const TALK_PROMPT_KIND_MENU_LIST: i32 = 5;
pub const TALK_PLAYER_TALK_LIST_OPTION_COUNT_OFFSET: u64 = 0x46_2A6C;
pub const GAME_STATE_RVA: u32 = 0x434_8610;
pub const CONTROL_MASK_SET_SLOT_ZERO_RVA: u32 = 0x3B_1370;
pub const CONTROL_MASK_TALK_SLOT_RVA: u32 = 0x435_AF88;
pub const TALK_TAG_RUN_ALL_RVA: u32 = 0xD_8950;
pub const MIRROR_OPEN_RETRY_MS: u64 = 1000;
pub const CHOICE_FOLLOW_GIVES_THE_GUEST_ITS_BUTTONS_AFTER_MS: u64 = 5000;
pub const FOLLOW_HOST_TALK_WAIT_MS: u64 = 2000;
pub const OWN_ENDED_TALK_NOT_MIRRORED_MS: u64 = 3000;
pub const UI_DRAW_PASS_RVA: u32 = 0xC_C8F0;
pub const TALK_JOIN_RANGE_UNITS: f32 = 15.0;
pub const TALK_LEAVE_EXTRA_UNITS: f32 = 2.0;
pub const CONTROL_MASK_SET_SLOT_ONE_RVA: u32 = 0x3B_1950;
pub const WATCHER_HOLD_MASK: u32 = 0xA0;
pub const TALK_EXPAND_TAGS_RVA: u32 = 0xD_AB50;
pub const TALK_SPEAKER_NAME_RVA: u32 = 0x4E_2D90;
pub const PLAYER_NAME_TAG: &str = "<NIER>";

mod prompt_vote;
mod skip_prompts;
mod start_gate;
mod tracker;

pub use prompt_vote::{PopVerdict, SentAnswer, SkipPromptVote, VoteEvent, VoteStep, VoteView};
pub use skip_prompts::{
    PROMPTS_WHOSE_ANSWER_SKIPS_A_SHARED_SEQUENCE, PromptAnswer, SkipPrompt, skip_prompt,
    skip_prompt_answer, skip_prompt_ended_by,
};
pub use start_gate::{SceneStartGate, StartVerdict};
pub use tracker::{CutsceneSkipTracker, SkipStep, SkipVerdict};

pub const SKIP_UPDATE_RVA: u32 = 0x3B_FF40;
pub const SKIP_MANAGER_ARMED_OFFSET: u64 = 0;
pub const SKIP_MANAGER_COUNTDOWN_OFFSET: u64 = 4;
pub const SKIP_MANAGER_LABEL_OFFSET: u64 = 8;
pub const SKIP_MANAGER_CONFIRMED_OFFSET: u64 = 0x108;
pub const SKIP_MANAGER_ALLOWED_OFFSET: u64 = 0x109;
pub const SKIP_COUNTDOWN_IDLE: f32 = -1.0;
pub const SKIP_CONFIRM_WINDOW_BELOW_SECONDS: f32 = 3.8;
pub const SKIP_PROMPT_UI_RVA: u32 = 0x277_E230;
pub const SKIP_PROMPT_HIDE_RVA: u32 = 0xC_C650;
pub const MAIN_MENU_STATE_RVA: u32 = 0x4B2_4AA0;
pub const MAIN_MENU_IS_OPEN_RVA: u32 = 0xC_A1C0;
pub const SKIP_MODAL_OBJECT_RVA: u32 = 0x122_0540;
pub const SKIP_MODAL_IS_SHOWN_RVA: u32 = 0x3D_5B20;
pub const CONFIRM_PAD_OBJECT_RVA: u32 = 0x443_E400;
pub const CONFIRM_PAD_PRESSED_RVA: u32 = 0x3D_44B0;
pub const CONFIRM_PAD_BUTTON_MASK: u32 = 0x100;
pub const CONFIRM_PAD_MODE: u64 = 2;
pub const CONFIRM_KEYBOARD_OBJECT_RVA: u32 = 0x443_E394;
pub const CONFIRM_KEYBOARD_PRESSED_RVA: u32 = 0x3D_25F0;
pub const TRIGGER_MANAGER_RVA: u32 = 0x122_D440;
pub const TRIGGER_FIND_NAMED_EVENT_RVA: u32 = 0x41_1C20;
pub const TRIGGER_MARK_READY_RVA: u32 = 0x41_3A70;
pub const EVENT_SKIP_TAKEN_BYTE_RVA: u32 = 0x38E_2E04;
pub const PARTNER_ASK_LIFETIME_MS: u64 = 2_000;
pub const ASK_RESEND_INTERVAL_MS: u64 = 500;
pub const PARTNER_SNAPSHOT_STALE_MS: u64 = 3_000;
pub const SKIP_PROMPT_VOTE_GIVES_UP_AFTER_MS: u64 = 30_000;
pub const SCENE_START_HOLD_TIMEOUT_MS: u64 = 15_000;
pub const SCENE_START_HOLD_RESTART_GAP_MS: u64 = 1_000;
pub const SCENE_ITEM_UPDATE_RVA: u32 = 0x42_9440;
pub const SCENE_ITEM_STATE_OFFSET: u64 = 0x18;
pub const SCENE_ITEM_SCENE_ID_OFFSET: u64 = 0x64;
pub const SCENE_ITEM_STATE_NOT_STARTED: u32 = 0;
pub const SCENE_ITEM_STATE_STARTING: u32 = 0x60;
pub const EVENT_CUT_LOADED_BYTE_RVA: u32 = 0x38E_2E01;

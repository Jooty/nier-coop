mod calls;
mod hooks;
mod state;

pub use hooks::install;
pub use state::{
    arm_game_over, note_posted_message, note_rewound, rewind_count, take_rewound_notice,
};

pub(crate) use calls::{rewind_now, take_backup};

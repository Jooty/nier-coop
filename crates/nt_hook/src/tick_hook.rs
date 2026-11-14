mod game_thread_tick;

pub use game_thread_tick::{
    install_game_thread_tick, install_game_thread_tick_with_result, is_draining,
};

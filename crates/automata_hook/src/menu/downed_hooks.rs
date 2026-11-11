/// What the Fixes section reads and asks about a downed hero; the game side fills it in on Windows.
#[derive(Debug, Clone, Copy)]
pub struct DownedHooks {
    pub hero_is_down: fn() -> bool,
    pub ask_to_give_up: fn(),
}

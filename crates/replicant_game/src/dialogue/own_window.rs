/// Whether the talk in this player's own window is this game's own copy. The game marks a talk
/// it starts in control slot one; a talk it chains in the same window can leave slot one clear,
/// and is still the own copy until the window closes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct OwnTalkWindow {
    had_own_talk: bool,
}

impl OwnTalkWindow {
    pub const fn new() -> Self {
        Self {
            had_own_talk: false,
        }
    }

    /// Once per tick: true while the open window shows a talk this game started or chained.
    pub fn observe(&mut self, own_window_open: bool, started_own_talk: bool) -> bool {
        if !own_window_open {
            self.had_own_talk = false;
            return false;
        }
        self.had_own_talk |= started_own_talk;
        self.had_own_talk
    }
}

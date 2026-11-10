use crate::load_gate::{
    LOADING_SCREEN_SHOWN_STATE, UI_PART_ASKED_TO_PLAY_OUT_BIT, UI_PART_HAS_PLAYED_OUT_BIT,
};

/// The fields of the game's loading screen that say how far it is; `None` is a failed read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LoadingScreenReading {
    pub state: Option<i32>,
    pub step: Option<i32>,
    pub mode: Option<i32>,
    pub opening_part: Option<u64>,
    pub shown_part: Option<u64>,
    /// The flags of the shown part; `None` also when there is no such part.
    pub shown_part_flags: Option<u32>,
}

impl LoadingScreenReading {
    /// True when the loading screen is fully shown, its opening part is gone and its shown part
    /// has played out: only the request for the load's end keeps it up, and it goes away the same
    /// short time after that request on every PC. Values that cannot be read count as ready.
    pub fn awaits_only_the_request(&self) -> bool {
        let (Some(state), Some(opening_part), Some(shown_part)) =
            (self.state, self.opening_part, self.shown_part)
        else {
            return true;
        };
        if state != LOADING_SCREEN_SHOWN_STATE || opening_part != 0 {
            return false;
        }
        shown_part == 0
            || self
                .shown_part_flags
                .is_none_or(|flags| flags & UI_PART_HAS_PLAYED_OUT_BIT != 0)
    }

    pub fn describe(&self) -> String {
        let number =
            |value: Option<i32>| value.map_or("unread".to_owned(), |value| value.to_string());
        let part = |part: Option<u64>| match part {
            None => "unread",
            Some(0) => "none",
            Some(_) => "there",
        };
        let flag = |bit: u32| match self.shown_part_flags {
            None => "unread",
            Some(flags) if flags & bit != 0 => "yes",
            Some(_) => "no",
        };
        format!(
            "state {}, step {}, mode {}, opening part {}, shown part {} (asked to play out: {}, played out: {})",
            number(self.state),
            number(self.step),
            number(self.mode),
            part(self.opening_part),
            part(self.shown_part),
            flag(UI_PART_ASKED_TO_PLAY_OUT_BIT),
            flag(UI_PART_HAS_PLAYED_OUT_BIT)
        )
    }
}

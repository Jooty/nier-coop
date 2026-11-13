use crate::dialogue::{
    TALK_DATA_LINE_COUNT_OFFSET, TALK_PLAYER_DRAWS_TEXT_STATES, TALK_WIDGET_NODE_OFFSET,
    TALK_WINDOW_CURRENT_LINE_OFFSET, TALK_WINDOW_CURRENT_TALK_OFFSET, TALK_WINDOW_MAY_SHOW_OFFSET,
    TALK_WINDOW_PLAYER_OFFSET, TALK_WINDOW_RVA, TALK_WINDOW_WIDGET_OFFSET,
    WIDGET_HANDLE_ENTRY_BYTES, WIDGET_HANDLE_ENTRY_GENERATION_OFFSET, WIDGET_HANDLE_NONE,
    WIDGET_HANDLE_TABLE_RVA, WIDGET_NODE_HANDLE_OFFSET, WIDGET_OBJECT_DRAWN_BIT,
    WIDGET_OBJECT_FLAGS_OFFSET,
};
use crate::marker::UI_DIALOG_STATE_RVA;
use crate::memory::{GameMemory, MemoryError};

/// What the talk window really puts on screen, read back from the window itself: the draw pass
/// shows its widget and text only when the window is open, its may-show flag is on, no main
/// menu covers it, and its talk player is in a text state on a line the talk has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TalkWindowScreen {
    pub window_state: i32,
    pub may_show: bool,
    pub main_menu_open: bool,
    pub widget_drawn: Option<bool>,
    pub player_state: i32,
    pub line: i32,
    pub line_count: Option<i32>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TalkScreenVerdict {
    Closed,
    HiddenByMenu,
    HiddenByMayShowFlag,
    NoTextYet,
    WidgetNotDrawn,
    OnScreen,
}

/// Logs a talk's screen verdict once it has had time to draw, then again only when it changes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScreenReport {
    talk: Option<(u32, u64)>,
    reported: Option<TalkScreenVerdict>,
}

impl TalkWindowScreen {
    pub fn verdict(&self) -> TalkScreenVerdict {
        if self.window_state == 0 {
            TalkScreenVerdict::Closed
        } else if self.main_menu_open {
            TalkScreenVerdict::HiddenByMenu
        } else if !self.may_show {
            TalkScreenVerdict::HiddenByMayShowFlag
        } else if !TALK_PLAYER_DRAWS_TEXT_STATES.contains(&self.player_state)
            || self.line < 0
            || self.line_count.is_none_or(|count| self.line >= count)
        {
            TalkScreenVerdict::NoTextYet
        } else if self.widget_drawn == Some(false) {
            TalkScreenVerdict::WidgetNotDrawn
        } else {
            TalkScreenVerdict::OnScreen
        }
    }

    /// The pause menu clears the may-show flag on entry and sets it on exit. Off with the window
    /// open and no menu up, an exit never ran (a load took the menu away) and nothing will show.
    pub fn may_show_flag_left_off(&self, pause_menu_open: bool) -> bool {
        self.window_state != 0 && !self.may_show && !self.main_menu_open && !pause_menu_open
    }

    pub fn describe(&self) -> String {
        let drawn = match self.widget_drawn {
            Some(true) => "drawn",
            Some(false) => "not drawn",
            None => "no widget",
        };
        let count = self
            .line_count
            .map_or_else(|| "?".to_owned(), |count| count.to_string());
        format!(
            "{}; window state {}, may-show flag {}, main menu {}, widget {drawn}, talk player state {}, line {} of {count}",
            self.verdict().describe(),
            self.window_state,
            u8::from(self.may_show),
            if self.main_menu_open {
                "open"
            } else {
                "closed"
            },
            self.player_state,
            self.line,
        )
    }
}

impl TalkScreenVerdict {
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Closed => "the window is closed",
            Self::HiddenByMenu => "hidden, a main menu covers it",
            Self::HiddenByMayShowFlag => "hidden, its may-show flag is off (nothing is drawn)",
            Self::NoTextYet => "open, no line on screen yet",
            Self::WidgetNotDrawn => "open, its widget was not drawn",
            Self::OnScreen => "on screen with its text",
        }
    }
}

impl ScreenReport {
    pub const fn new() -> Self {
        Self {
            talk: None,
            reported: None,
        }
    }

    /// Once per tick with the talk the window shows (if any) and its verdict now.
    pub fn observe(
        &mut self,
        talk_id: Option<u32>,
        verdict: TalkScreenVerdict,
        now_ms: u64,
        settle_ms: u64,
    ) -> Option<(u32, TalkScreenVerdict)> {
        let Some(talk_id) = talk_id else {
            *self = Self::new();
            return None;
        };
        let opened_ms = match self.talk {
            Some((seen, opened_ms)) if seen == talk_id => opened_ms,
            _ => {
                self.talk = Some((talk_id, now_ms));
                self.reported = None;
                now_ms
            }
        };
        if now_ms.saturating_sub(opened_ms) < settle_ms || self.reported == Some(verdict) {
            return None;
        }
        self.reported = Some(verdict);
        Some((talk_id, verdict))
    }
}

pub fn read_talk_window_screen<M: GameMemory>(memory: &M) -> Result<TalkWindowScreen, MemoryError> {
    let base = memory.module_base();
    let window = base + u64::from(TALK_WINDOW_RVA);
    let talk = memory.read_u64(window + TALK_WINDOW_CURRENT_TALK_OFFSET)?;
    let line_count = if talk == 0 {
        None
    } else {
        Some(memory.read_i32(talk + TALK_DATA_LINE_COUNT_OFFSET)?)
    };
    Ok(TalkWindowScreen {
        window_state: memory.read_i32(window)?,
        may_show: memory.read_u8(window + TALK_WINDOW_MAY_SHOW_OFFSET)? != 0,
        main_menu_open: memory.read_i32(base + u64::from(UI_DIALOG_STATE_RVA))? != 0,
        widget_drawn: read_widget_drawn(memory, window + TALK_WINDOW_WIDGET_OFFSET)?,
        player_state: memory.read_i32(window + TALK_WINDOW_PLAYER_OFFSET)?,
        line: memory.read_i32(window + TALK_WINDOW_CURRENT_LINE_OFFSET)?,
        line_count,
    })
}

pub fn put_may_show_flag_back<M: GameMemory>(memory: &mut M) -> Result<(), MemoryError> {
    let window = memory.module_base() + u64::from(TALK_WINDOW_RVA);
    memory.write_u8(window + TALK_WINDOW_MAY_SHOW_OFFSET, 1)
}

fn read_widget_drawn<M: GameMemory>(memory: &M, widget: u64) -> Result<Option<bool>, MemoryError> {
    let node = memory.read_u64(widget + TALK_WIDGET_NODE_OFFSET)?;
    if node == 0 {
        return Ok(None);
    }
    let handle = u32::from_le_bytes(memory.read_array::<4>(node + WIDGET_NODE_HANDLE_OFFSET)?);
    let generation = handle >> 16;
    if generation == WIDGET_HANDLE_NONE {
        return Ok(None);
    }
    // The handle's low half indexes the game's object table; its high half must match the entry.
    let entry = memory.module_base()
        + u64::from(WIDGET_HANDLE_TABLE_RVA)
        + u64::from(handle & 0xFFFF) * WIDGET_HANDLE_ENTRY_BYTES;
    let entry_generation =
        u32::from_le_bytes(memory.read_array::<4>(entry + WIDGET_HANDLE_ENTRY_GENERATION_OFFSET)?);
    let object = memory.read_u64(entry)?;
    if entry_generation != generation || object == 0 {
        return Ok(None);
    }
    let flags = memory.read_u8(object + WIDGET_OBJECT_FLAGS_OFFSET)?;
    Ok(Some(flags & WIDGET_OBJECT_DRAWN_BIT != 0))
}

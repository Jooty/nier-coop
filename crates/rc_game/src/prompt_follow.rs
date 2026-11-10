pub const PROMPT_PAGE_SHOWN_DIALOG_STATE: i32 = 0x5;
pub const MAX_PROMPT_CLOSE_TRIES_PER_PAGE: u32 = 3;
pub const PROMPT_CLOSE_RETRY_AFTER_MS: u64 = 500;
pub const PROMPT_BEHIND_STREAM_GRACE_MS: u64 = 500;
pub const HOST_DISMISSAL_FOLLOWED_OUTSIDE_A_FIGHT_FOR_MS: u64 = 30_000;
pub const DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS: u64 = 1_000;
pub const UI_DIALOG_OPEN_FUNCTION_RVA: u32 = 0x45_F640;
pub const SCRIPT_DIALOG_OPENED_BY_THE_TUTORIAL_OPENER_WIRE_CODE: u8 = 0;

/// A waiting script native that shows a page of the shared UI dialog and waits until it closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScriptDialogNative {
    Start2DInfo,
    StartInfoItem,
    StartInfoMoney,
    StartInfoQuest,
    StartInfoWord,
    StartInfoTutorial,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromptView {
    pub stream_active: bool,
    pub dialog_state: i32,
    pub pause_menu_open: bool,
    pub now_ms: u64,
    /// This game's phase index; the counts start over when it changes.
    pub phase: Option<u16>,
    pub partner_in_same_phase: bool,
    /// The script slot is parked in one of the six dialog waits (see `dialog_wait_parked`).
    pub parked_in_dialog_wait: bool,
    /// A later host stream entry has waited behind the script slot this long.
    pub entry_behind_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptPageId {
    /// The waiting native that opened the dialog; `None` for the tutorial opener called outside one.
    pub native: Option<ScriptDialogNative>,
    /// The dialog id UiDialog::Open was given.
    pub kind: i32,
    /// The tutorial id for a tutorial prompt, else 0.
    pub id: i32,
    pub page: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptDismissal {
    pub page: PromptPageId,
    pub times: u32,
    /// Counted inside the fight stream; else outside one, in a phase both players are in.
    pub in_fight: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptClose {
    pub page: PromptPageId,
    pub try_number: u32,
}

/// A close of the shown page because a later host entry waits behind the parked dialog wait.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PromptCatchUpClose {
    pub try_number: u32,
    pub entry_behind_ms: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PromptStep {
    pub dismissed: Option<PromptDismissal>,
    pub close: Option<PromptClose>,
    pub gave_up_on: Option<PromptPageId>,
    pub catch_up_close: Option<PromptCatchUpClose>,
    pub catch_up_gave_up: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct CloseAttempt {
    tries: u32,
    last_try_ms: u64,
    gave_up_reported: bool,
}

enum CloseTry {
    Now(u32),
    GiveUp,
    Later,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct OpenIdentity {
    native: Option<ScriptDialogNative>,
    kind: i32,
    id: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct HostCount {
    page: PromptPageId,
    times: u32,
    in_fight: bool,
    received_ms: u64,
}

#[derive(Clone, Debug, Default)]
pub struct PromptFollow {
    stream_was_active: bool,
    phase: Option<u16>,
    current: Option<OpenIdentity>,
    pending_open: Option<OpenIdentity>,
    pages_shown: u32,
    page_shown: bool,
    own_dismissed: Vec<(PromptPageId, u32)>,
    host_dismissed: Vec<HostCount>,
    attempt: Option<CloseAttempt>,
    last_now_ms: u64,
}

impl ScriptDialogNative {
    pub const ALL: [Self; 6] = [
        Self::Start2DInfo,
        Self::StartInfoItem,
        Self::StartInfoMoney,
        Self::StartInfoQuest,
        Self::StartInfoWord,
        Self::StartInfoTutorial,
    ];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Start2DInfo => "_Start2DInfo",
            Self::StartInfoItem => "_StartInfoItem",
            Self::StartInfoMoney => "_StartInfoMoney",
            Self::StartInfoQuest => "_StartInfoQuest",
            Self::StartInfoWord => "_StartInfoWord",
            Self::StartInfoTutorial => "_StartInfoTutorial",
        }
    }

    /// The function the script step polls while this native waits (manager + 0xD8 in, a byte
    /// out that is non-zero while it waits).
    pub const fn poll_rva(self) -> u32 {
        match self {
            Self::Start2DInfo => 0x70_7370,
            Self::StartInfoItem => 0x70_83D0,
            Self::StartInfoMoney => 0x70_8490,
            Self::StartInfoQuest => 0x70_8530,
            Self::StartInfoWord => 0x70_8670,
            Self::StartInfoTutorial => 0x70_85D0,
        }
    }

    pub const fn wire_code(self) -> u8 {
        match self {
            Self::Start2DInfo => 1,
            Self::StartInfoItem => 2,
            Self::StartInfoMoney => 3,
            Self::StartInfoQuest => 4,
            Self::StartInfoWord => 5,
            Self::StartInfoTutorial => 6,
        }
    }

    pub fn from_wire_code(code: u8) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|native| native.wire_code() == code)
    }

    /// Outside a fight only story pop-ups follow the host; item, money, quest and word windows
    /// stay each player's own there.
    pub const fn follows_outside_a_fight(self) -> bool {
        matches!(self, Self::Start2DInfo | Self::StartInfoTutorial)
    }

    /// The tutorial wait opens through UiDialog::OpenTutorial, which names the prompt itself.
    pub const fn opens_through_its_poll(self) -> bool {
        !matches!(self, Self::StartInfoTutorial)
    }
}

impl PromptPageId {
    pub fn wire_native_code(self) -> u8 {
        self.native.map_or(
            SCRIPT_DIALOG_OPENED_BY_THE_TUTORIAL_OPENER_WIRE_CODE,
            ScriptDialogNative::wire_code,
        )
    }
}

impl PromptFollow {
    pub const fn new() -> Self {
        Self {
            stream_was_active: false,
            phase: None,
            current: None,
            pending_open: None,
            pages_shown: 0,
            page_shown: false,
            own_dismissed: Vec::new(),
            host_dismissed: Vec::new(),
            attempt: None,
            last_now_ms: 0,
        }
    }

    pub fn dismissed_times(&self, page: PromptPageId) -> u32 {
        count_of(&self.own_dismissed, page)
    }

    pub fn host_dismissed_times(&self, page: PromptPageId) -> u32 {
        self.host_count(page, self.stream_was_active)
            .map_or(0, |count| count.times)
    }

    /// A count of the host's taken inside its fight stream.
    pub fn receive_host_dismissal(&mut self, dismissal: PromptDismissal) {
        let received_ms = self.last_now_ms;
        self.receive_host_dismissal_at(dismissal, received_ms);
    }

    pub fn receive_host_dismissal_at(&mut self, dismissal: PromptDismissal, received_ms: u64) {
        let count = HostCount {
            page: dismissal.page,
            times: dismissal.times,
            in_fight: dismissal.in_fight,
            received_ms,
        };
        if let Some(known) = self
            .host_dismissed
            .iter_mut()
            .find(|known| known.page == count.page && known.in_fight == count.in_fight)
        {
            *known = count;
        } else {
            self.host_dismissed.push(count);
        }
    }

    /// The tutorial opener showed `kind`/`id` outside a known dialog wait.
    pub const fn note_open(&mut self, kind: i32, id: i32) {
        self.note_dialog_open(None, kind, id);
    }

    pub const fn note_dialog_open(
        &mut self,
        native: Option<ScriptDialogNative>,
        kind: i32,
        id: i32,
    ) {
        let identity = OpenIdentity { native, kind, id };
        if self.page_shown {
            self.pending_open = Some(identity);
        } else {
            self.current = Some(identity);
            self.pages_shown = 0;
        }
    }

    pub fn tick(&mut self, view: PromptView) -> PromptStep {
        let mut step = PromptStep::default();
        self.last_now_ms = view.now_ms;
        if view.phase != self.phase {
            self.phase = view.phase;
            self.own_dismissed.clear();
            self.host_dismissed.clear();
            self.attempt = None;
        }
        if view.stream_active != self.stream_was_active {
            let left_in_fight = self.stream_was_active;
            self.stream_was_active = view.stream_active;
            self.own_dismissed.clear();
            self.attempt = None;
            // Counts of the scope left are stale; a host count of the new one may come first.
            self.host_dismissed
                .retain(|count| count.in_fight != left_in_fight);
        }
        let page_open = view.dialog_state == PROMPT_PAGE_SHOWN_DIALOG_STATE;
        let page_shown_now = page_open && !view.pause_menu_open;
        if page_shown_now && !self.page_shown {
            self.page_shown = true;
            self.pages_shown = self.pages_shown.saturating_add(1);
        } else if self.page_shown && !page_open {
            self.page_shown = false;
            self.attempt = None;
            step.dismissed = self.count_dismissal(&view);
        }
        if !self.page_shown
            && let Some(identity) = self.pending_open.take()
        {
            self.current = Some(identity);
            self.pages_shown = 0;
        }
        if view.dialog_state == 0 && !self.page_shown {
            self.current = None;
            self.pages_shown = 0;
        }
        if !page_shown_now {
            return step;
        }
        if let Some(page) = self.shown_page()
            && self.in_scope(page, &view)
            && count_of(&self.own_dismissed, page) < self.host_times_to_follow(page, &view)
        {
            match self.next_close_try(view.now_ms) {
                CloseTry::Now(try_number) => step.close = Some(PromptClose { page, try_number }),
                CloseTry::GiveUp => step.gave_up_on = Some(page),
                CloseTry::Later => {}
            }
            return step;
        }
        if view.stream_active
            && view.parked_in_dialog_wait
            && let Some(entry_behind_ms) = view.entry_behind_ms
            && entry_behind_ms > PROMPT_BEHIND_STREAM_GRACE_MS
        {
            match self.next_close_try(view.now_ms) {
                CloseTry::Now(try_number) => {
                    step.catch_up_close = Some(PromptCatchUpClose {
                        try_number,
                        entry_behind_ms,
                    });
                }
                CloseTry::GiveUp => step.catch_up_gave_up = true,
                CloseTry::Later => {}
            }
        }
        step
    }

    fn shown_page(&self) -> Option<PromptPageId> {
        let identity = self.current?;
        Some(PromptPageId {
            native: identity.native,
            kind: identity.kind,
            id: identity.id,
            page: self.pages_shown,
        })
    }

    fn in_scope(&self, page: PromptPageId, view: &PromptView) -> bool {
        view.stream_active
            || (view.partner_in_same_phase
                && page
                    .native
                    .is_some_and(ScriptDialogNative::follows_outside_a_fight))
    }

    fn host_count(&self, page: PromptPageId, in_fight: bool) -> Option<&HostCount> {
        self.host_dismissed
            .iter()
            .find(|count| count.page == page && count.in_fight == in_fight)
    }

    fn host_times_to_follow(&self, page: PromptPageId, view: &PromptView) -> u32 {
        let Some(count) = self.host_count(page, view.stream_active) else {
            return 0;
        };
        let fresh = view.stream_active
            || view.now_ms.saturating_sub(count.received_ms)
                <= HOST_DISMISSAL_FOLLOWED_OUTSIDE_A_FIGHT_FOR_MS;
        if fresh { count.times } else { 0 }
    }

    fn count_dismissal(&mut self, view: &PromptView) -> Option<PromptDismissal> {
        let page = self.shown_page()?;
        if !self.in_scope(page, view) {
            return None;
        }
        let times = count_of(&self.own_dismissed, page).saturating_add(1);
        set_count(&mut self.own_dismissed, page, times);
        Some(PromptDismissal {
            page,
            times,
            in_fight: view.stream_active,
        })
    }

    fn next_close_try(&mut self, now_ms: u64) -> CloseTry {
        let Some(attempt) = &mut self.attempt else {
            self.attempt = Some(CloseAttempt {
                tries: 1,
                last_try_ms: now_ms,
                gave_up_reported: false,
            });
            return CloseTry::Now(1);
        };
        if now_ms.saturating_sub(attempt.last_try_ms) < PROMPT_CLOSE_RETRY_AFTER_MS {
            return CloseTry::Later;
        }
        if attempt.tries >= MAX_PROMPT_CLOSE_TRIES_PER_PAGE {
            if attempt.gave_up_reported {
                return CloseTry::Later;
            }
            attempt.gave_up_reported = true;
            return CloseTry::GiveUp;
        }
        attempt.tries += 1;
        attempt.last_try_ms = now_ms;
        CloseTry::Now(attempt.tries)
    }
}

fn count_of(counts: &[(PromptPageId, u32)], page: PromptPageId) -> u32 {
    counts
        .iter()
        .find(|(known, _)| *known == page)
        .map_or(0, |(_, times)| *times)
}

fn set_count(counts: &mut Vec<(PromptPageId, u32)>, page: PromptPageId, times: u32) {
    if let Some(entry) = counts.iter_mut().find(|(known, _)| *known == page) {
        entry.1 = times;
    } else {
        counts.push((page, times));
    }
}

/// Whether the script slot is parked in a dialog wait: its last poll still waited, and came
/// within `DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS`.
pub fn dialog_wait_parked(last_still_waiting_poll_ms: Option<u64>, now_ms: u64) -> bool {
    last_still_waiting_poll_ms.is_some_and(|polled_ms| {
        now_ms.saturating_sub(polled_ms) <= DIALOG_WAIT_PARKED_WHILE_POLLED_WITHIN_MS
    })
}

fn page_label(page: PromptPageId) -> String {
    match page.native {
        Some(native) => format!(
            "{} {}/{} page {}",
            native.name(),
            page.kind,
            page.id,
            page.page
        ),
        None => format!("{}/{} page {}", page.kind, page.id, page.page),
    }
}

pub fn format_prompt_opened(native: Option<ScriptDialogNative>, kind: i32, id: i32) -> String {
    let what = native.map_or("script prompt", ScriptDialogNative::name);
    format!("prompt follow: {what} {kind}/{id} opened")
}

pub fn format_prompt_follow_close(close: PromptClose) -> String {
    format!(
        "prompt follow: closing your {}, the host dismissed it",
        page_label(close.page)
    )
}

pub fn format_prompt_follow_gave_up(page: PromptPageId) -> String {
    format!(
        "prompt follow: closing your {} did not work after {MAX_PROMPT_CLOSE_TRIES_PER_PAGE} tries, left alone",
        page_label(page)
    )
}

pub fn format_prompt_catch_up_close(
    close: PromptCatchUpClose,
    native: Option<ScriptDialogNative>,
    handler: &str,
) -> String {
    let native = native.map_or("a dialog wait", ScriptDialogNative::name);
    format!(
        "prompt follow: closing your page, the host moved on ({native} in {handler}, try {}, a host entry waited {} ms behind it)",
        close.try_number, close.entry_behind_ms
    )
}

pub fn format_prompt_catch_up_gave_up(handler: &str) -> String {
    format!(
        "prompt follow: closing your page in {handler} did not work after {MAX_PROMPT_CLOSE_TRIES_PER_PAGE} tries, left alone"
    )
}

pub fn format_host_dismissal(dismissal: PromptDismissal) -> String {
    let scope = if dismissal.in_fight {
        "in the fight"
    } else {
        "outside a fight"
    };
    format!(
        "prompt follow: the host dismissed {} ({} times, {scope})",
        page_label(dismissal.page),
        dismissal.times
    )
}

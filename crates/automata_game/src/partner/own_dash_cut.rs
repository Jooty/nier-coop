const DASH_STATE: i32 = 0x12;
const SPRINT_STATE: i32 = 0x3;

/// What the local hero looked like when a state start took it out of a dash or a sprint.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnDashCut {
    pub from_state: i32,
    pub to_state: i32,
    pub held_buttons_now: u32,
    pub held_buttons_at_the_dash_start: Option<u32>,
    pub speed: f32,
    pub input_ignore_seconds: f32,
    pub inside_a_held_body_update: bool,
    pub caller_rvas: Vec<u32>,
}

pub fn is_a_dash_state(state: i32) -> bool {
    state == DASH_STATE || state == SPRINT_STATE
}

/// True when a start of `started` takes a body that is in `current` out of its dash or sprint.
pub fn start_cuts_a_dash(current: i32, started: i32) -> bool {
    is_a_dash_state(current) && !is_a_dash_state(started)
}

/// Held buttons of the dash's start that are no longer held: zero when nothing was let go.
pub fn buttons_let_go(cut: &OwnDashCut) -> Option<u32> {
    cut.held_buttons_at_the_dash_start
        .map(|at_start| at_start & !cut.held_buttons_now)
}

pub fn own_dash_cut_line(cut: &OwnDashCut, number: u32, at_most_a_minute: u32) -> String {
    let held_at_start = cut
        .held_buttons_at_the_dash_start
        .map_or_else(|| "not seen".to_owned(), |held| format!("{held:#X}"));
    let let_go = buttons_let_go(cut).map_or_else(
        || "unknown".to_owned(),
        |bits| {
            if bits == 0 {
                "none".to_owned()
            } else {
                format!("{bits:#X}")
            }
        },
    );
    let callers = if cut.caller_rvas.is_empty() {
        "none found".to_owned()
    } else {
        cut.caller_rvas
            .iter()
            .map(|rva| format!("{rva:#X}"))
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "own dash: the hero went from state {:#X} to {:#X}; held buttons {:#X} (at the dash's start {held_at_start}, let go since: {let_go}), speed {}, input ignore {} s, a held body's update was running: {}, game code on the stack {callers} (line {number}, at most {at_most_a_minute} a minute)",
        cut.from_state,
        cut.to_state,
        cut.held_buttons_now,
        cut.speed,
        cut.input_ignore_seconds,
        if cut.inside_a_held_body_update {
            "yes"
        } else {
            "no"
        },
    )
}

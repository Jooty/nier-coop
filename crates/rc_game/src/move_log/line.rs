use crate::move_log::{MoveLogDue, MoveSample, RateChange, state_name_for_rva};

pub struct MoveLineContext<'a> {
    pub due: MoveLogDue,
    pub action_set: &'a str,
    pub seed: &'a str,
    pub talk_window_open: bool,
    pub window_focused: bool,
}

pub fn format_move_line(context: &MoveLineContext, sample: &MoveSample) -> String {
    let due = context.due;
    let state = sample.state_rva.map_or_else(
        || "?".to_owned(),
        |rva| format!("{}({rva:#X})", state_name_for_rva(rva)),
    );
    format!(
        "move: {} +{}s action_set={} seed={} state={state} speed={} step_per_s={} pending={} \
         lock={} level={} limit={} block={} mode={} collision={} lockbytes={} rates={} mask={} \
         event_scene={} talk={} focused={} frame_ms={:.1} (max {}, {} frames)",
        due.reason.label(),
        due.seconds_in,
        context.action_set,
        context.seed,
        number(sample.speed),
        number(sample.applied_step_per_second),
        number(sample.displacement_pending),
        number(sample.state_lock),
        number(sample.state_level),
        number(sample.state_limit),
        number(sample.move_block),
        number(sample.damage_mode),
        bytes(sample.collision_flags),
        bytes(sample.lock_byte_flags),
        rates(sample.rate_changes.as_deref()),
        mask(sample.mask_slots),
        number(sample.event_scene.map(u8::from)),
        u8::from(context.talk_window_open),
        u8::from(context.window_focused),
        due.frame_ms_average,
        due.frame_ms_longest,
        due.frames,
    )
}

fn number<T: NumberText>(value: Option<T>) -> String {
    value.map_or_else(|| "?".to_owned(), NumberText::text)
}

trait NumberText {
    fn text(self) -> String;
}

impl NumberText for f32 {
    fn text(self) -> String {
        format!("{self:.2}")
    }
}

impl NumberText for i32 {
    fn text(self) -> String {
        self.to_string()
    }
}

impl NumberText for u8 {
    fn text(self) -> String {
        self.to_string()
    }
}

fn bytes(value: Option<[u8; 2]>) -> String {
    value.map_or_else(|| "?".to_owned(), |[a, b]| format!("{a}/{b}"))
}

fn mask(slots: Option<[u32; 4]>) -> String {
    slots.map_or_else(
        || "?".to_owned(),
        |[a, b, c, d]| format!("{a:#x}/{b:#x}/{c:#x}/{d:#x}"),
    )
}

fn rates(changes: Option<&[RateChange]>) -> String {
    match changes {
        None => "?".to_owned(),
        Some([]) => "all 1.0".to_owned(),
        Some(changes) => changes
            .iter()
            .map(|change| format!("{}:{}={:.2}", change.id, change.channel, change.value))
            .collect::<Vec<_>>()
            .join(","),
    }
}

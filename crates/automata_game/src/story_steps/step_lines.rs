use crate::load_gate::StoryPlace;
use crate::story_steps::{
    ObservedDone, STORY_STEP_GIVE_UP_MS, STORY_STEP_HOST_NOT_STARTED_WAIT_MS, SetDone, StepAnswer,
    StoryStepAsk, StoryStepDone, StoryStepsMode, WATCHED_STEP_COMPARE_AFTER_MS,
};
use crate::travel::JumpRole;

pub fn story_steps_on_line(mode: StoryStepsMode, classes: usize) -> String {
    match mode {
        StoryStepsMode::On => format!(
            "story steps: on, the guest's dones of mirrored story actions wait for the host's ({classes} classes in the table)"
        ),
        StoryStepsMode::Watch | StoryStepsMode::Off => format!(
            "story steps: watch, every done of a blocking story action is only logged and compared, nothing is held ({classes} classes in the table)"
        ),
    }
}

pub fn story_steps_off_line() -> String {
    "story steps: off, nothing is hooked".to_owned()
}

pub fn story_steps_start_differs_line(function: &str, address: u64) -> String {
    format!(
        "story steps: the {function} at {address:#X} does not start as noted, nothing is hooked"
    )
}

pub fn story_steps_hooks_missing_line() -> String {
    "story steps: a hook could not be made, story steps stay off".to_owned()
}

pub fn busy_passes_line(count: u32) -> String {
    format!("story steps: {count} state changes passed unseen while the machine was busy")
}

pub fn set_done_line(set: &SetDone, applied: bool) -> String {
    let outcome = if applied {
        "set done here"
    } else {
        "not running here, nothing set"
    };
    format!(
        "story steps: id {:#010X} ({}): {outcome}",
        set.id,
        set.why.label()
    )
}

pub(crate) fn role_label(role: JumpRole) -> &'static str {
    match role {
        JumpRole::Host => "host",
        JumpRole::Guest => "guest",
        JumpRole::Solo => "solo",
    }
}

pub(crate) fn mirrored_done_line(
    role: JumpRole,
    done: &ObservedDone,
    place: &StoryPlace,
    now_ms: u64,
) -> String {
    let by = if done.ours { " (set by this mod)" } else { "" };
    format!(
        "story steps: {} done id {:#010X} class {} ({}) value {} at {} t={now_ms} ms{by}",
        role_label(role),
        done.id,
        done.class_name(),
        done.class.map_or("none", |row| row.sort.label()),
        done.value,
        place.describe()
    )
}

pub(crate) fn unmirrored_done_line(
    role: JumpRole,
    done: &ObservedDone,
    place: &StoryPlace,
    now_ms: u64,
) -> String {
    let (sort, reason) = done.class.map_or(("none", "not in the table"), |row| {
        (row.sort.label(), row.reason)
    });
    format!(
        "story steps: {} done id {:#010X} class {} ({sort}) not mirrored: {reason}; at {} t={now_ms} ms",
        role_label(role),
        done.id,
        done.class_name(),
        place.describe()
    )
}

pub(crate) fn unknown_class_done_line(
    role: JumpRole,
    done: &ObservedDone,
    place: &StoryPlace,
    now_ms: u64,
) -> String {
    format!(
        "story steps: {} done id {:#010X} of a class not in the table (vftable rva {:#X}), not mirrored; at {} t={now_ms} ms",
        role_label(role),
        done.id,
        done.vftable_rva,
        place.describe()
    )
}

pub(crate) fn ask_answered_line(ask: &StoryStepAsk, how: &str) -> String {
    format!(
        "story steps: the guest asked for id {:#010X} at {}: {how}",
        ask.id,
        ask.place.describe()
    )
}

pub(crate) fn kept_done_line(id: u32) -> String {
    format!(
        "story steps: id {id:#010X} was taken done and stays done, a change out of done was refused"
    )
}

pub(crate) fn held_line(done: &ObservedDone, place: &StoryPlace, now_ms: u64) -> String {
    format!(
        "story steps: held this game's done of id {:#010X} class {} at {} t={now_ms} ms, asking the host",
        done.id,
        done.class_name(),
        place.describe()
    )
}

pub(crate) fn too_many_held_line(id: u32) -> String {
    format!("story steps: too many dones held already, the done of id {id:#010X} passes")
}

pub(crate) fn holding_line(count: usize, oldest_id: u32, waited_ms: u64) -> String {
    format!(
        "story steps: holding {count} done(s), the oldest id {oldest_id:#010X} for {waited_ms} ms"
    )
}

pub(crate) fn host_done_heard_line(done: &StoryStepDone, held_ms: Option<u64>) -> String {
    let held = held_ms.map_or(String::new(), |ms| {
        format!(" (this game's own was held {ms} ms)")
    });
    format!(
        "story steps: heard the host's done {} of id {:#010X} value {}{held}",
        done.number, done.id, done.value
    )
}

pub(crate) fn answer_heard_line(id: u32, answer: StepAnswer) -> String {
    format!(
        "story steps: the host answered for id {id:#010X}: {}; this game's done passes in {STORY_STEP_HOST_NOT_STARTED_WAIT_MS} ms without the host's",
        answer.label()
    )
}

pub(crate) fn already_done_here_line(id: u32) -> String {
    format!("story steps: the host's done of id {id:#010X} is already done here")
}

pub(crate) fn gave_up_line(id: u32, waited_ms: u64, answered: bool) -> String {
    let why = if answered {
        "the host's action had not started"
    } else {
        "no answer from the host"
    };
    format!(
        "story steps: GAVE UP on id {id:#010X} after {waited_ms} ms ({why}), this game's own done passes"
    )
}

pub(crate) fn link_lost_line(held: usize) -> String {
    format!("story steps: the link is lost, {held} held done(s) pass")
}

pub(crate) fn told_other_place_line(done: &StoryStepDone, place: &StoryPlace) -> String {
    format!(
        "story steps: the host's done {} of id {:#010X} is for {}, this game is at {}: kept until this game is there",
        done.number,
        done.id,
        done.place.describe(),
        place.describe()
    )
}

pub(crate) fn places_differ_line(id: u32, own: &StoryPlace, host: &StoryPlace) -> String {
    format!(
        "story steps: this game's done of id {id:#010X} passes: the host is at {}, this game at {}",
        host.describe(),
        own.describe()
    )
}

pub(crate) fn lists_emptied_line(why: &str) -> String {
    format!("story steps: lists emptied ({why})")
}

pub(crate) fn reset_forgotten_line(id: u32) -> String {
    format!("story steps: id {id:#010X} was reset here, forgotten")
}

pub(crate) fn watch_own_before_line(id: u32, class: &str, ms: u64) -> String {
    format!(
        "story steps: watch: this game's done of id {id:#010X} ({class}) came {ms} ms before the host's: on would have held it {ms} ms"
    )
}

pub(crate) fn watch_own_after_line(id: u32, class: &str, ms: u64) -> String {
    format!(
        "story steps: watch: this game's done of id {id:#010X} ({class}) came {ms} ms after the host's: on would not hold it"
    )
}

pub(crate) fn watch_host_not_reached_line(id: u32, place: &StoryPlace) -> String {
    format!(
        "story steps: watch: the host's done of id {id:#010X} is not reached here within {WATCHED_STEP_COMPARE_AFTER_MS} ms (at {})",
        place.describe()
    )
}

pub(crate) fn watch_own_without_host_line(id: u32, class: &str) -> String {
    format!(
        "story steps: watch: no host done for this game's done of id {id:#010X} ({class}) within {STORY_STEP_GIVE_UP_MS} ms: on would have held it {STORY_STEP_GIVE_UP_MS} ms, then let it pass"
    )
}

use crate::event_follow::{EventId, GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS, StartOrigin};

pub fn started_here_line(id: EventId, origin: StartOrigin) -> String {
    format!("event: {} started here {}", id.describe(), origin.label())
}

pub fn ended_here_line(id: EventId, skipped: bool, played_ms: Option<u64>) -> String {
    let how = if skipped { "skipped" } else { "ended" };
    match played_ms {
        Some(played_ms) => format!("event: {} {how} here after {played_ms} ms", id.describe()),
        None => format!("event: {} {how} here", id.describe()),
    }
}

pub fn told_line(id: EventId, skipped: Option<bool>) -> String {
    match skipped {
        None => format!("event: told the partner the start of {}", id.describe()),
        Some(true) => format!("event: told the partner {} was skipped", id.describe()),
        Some(false) => format!("event: told the partner the end of {}", id.describe()),
    }
}

pub fn replay_skipped_line(id: EventId) -> String {
    format!(
        "event: {} was started here for the partner before, this game's own play of it is skipped at once and not told",
        id.describe()
    )
}

pub fn play_held_line(id: EventId) -> String {
    format!(
        "event: this game's script plays {} before the host started it, held and asked of the host",
        id.describe()
    )
}

pub fn play_passed_line(id: EventId) -> String {
    format!(
        "event: this game's script plays {}, which the host started, it passes",
        id.describe()
    )
}

pub fn held_waiting_line(id: EventId, waited_ms: u64) -> String {
    format!(
        "event: {} held for {waited_ms} ms, still no start from the host (asked again)",
        id.describe()
    )
}

pub fn held_released_line(id: EventId, waited_ms: u64, why: &str) -> String {
    format!(
        "event: {} released after {waited_ms} ms held: {why}",
        id.describe()
    )
}

pub fn held_gave_up_line(id: EventId, waited_ms: u64) -> String {
    format!(
        "event: NO START OF {} FROM THE HOST IN {waited_ms} MS, it plays here alone",
        id.describe()
    )
}

pub fn host_start_heard_line(id: EventId, how: &str) -> String {
    format!("event: the host started {} ({how})", id.describe())
}

pub fn host_end_heard_line(id: EventId, skipped: bool) -> String {
    let how = if skipped { "skipped" } else { "ended" };
    format!("event: the host {how} {}", id.describe())
}

pub fn skipped_for_the_host_line(id: EventId) -> String {
    format!(
        "event: {} is skipped here because the host's was skipped",
        id.describe()
    )
}

pub fn outlast_started_line(id: EventId) -> String {
    format!(
        "event: the host's {} ended, this game's may run on for up to {} ms",
        id.describe(),
        GUEST_EVENT_MAY_OUTLAST_THE_HOSTS_BY_MS
    )
}

pub fn outlast_skip_line(id: EventId) -> String {
    format!(
        "event: {} ran on too long after the host's end, skipped here",
        id.describe()
    )
}

pub fn skip_swallowed_line(id: EventId) -> String {
    format!(
        "event: the player skipped {}, which the host also plays: swallowed and asked of the host",
        id.describe()
    )
}

pub fn skip_ask_gave_up_line(id: EventId, waited_ms: u64) -> String {
    format!(
        "event: no end of {} from the host {waited_ms} ms after the skip ask, skipped here",
        id.describe()
    )
}

pub fn follow_start_line(id: EventId, played_ms: u64) -> String {
    format!(
        "event: the host has played {} for {played_ms} ms and this game has not, starting it here by its DataId",
        id.describe()
    )
}

pub fn follow_waits_line(id: EventId, played_ms: u64, reason: &str) -> String {
    format!(
        "event: the host has played {} for {played_ms} ms, not started here: {reason}",
        id.describe()
    )
}

pub fn ask_start_made_line(id: EventId) -> String {
    format!(
        "event: the guest asked for {}, starting it here by its DataId",
        id.describe()
    )
}

pub fn ask_retold_line(id: EventId, skipped: Option<bool>) -> String {
    let what = match skipped {
        None => "it plays here, its start is told again",
        Some(true) => "it was skipped here, its start and skip are told again",
        Some(false) => "it ended here, its start and end are told again",
    };
    format!("event: the guest asked for {}: {what}", id.describe())
}

pub fn ask_refused_line(id: EventId, reason: &str) -> String {
    format!(
        "event: the guest asked for {}, refused: {reason}",
        id.describe()
    )
}

pub fn ask_skip_made_line(id: EventId) -> String {
    format!(
        "event: the guest skipped {}, skipped here for both",
        id.describe()
    )
}

pub fn ask_skip_ignored_line(id: EventId) -> String {
    format!(
        "event: the guest asked to skip {}, which does not play here: ignored",
        id.describe()
    )
}

pub fn run_released_line(id: EventId) -> String {
    format!(
        "event: {} started here for the partner has ended, released",
        id.describe()
    )
}

pub fn run_gave_up_line(id: EventId, waited_ms: u64, reason: &str) -> String {
    format!(
        "event: {} started here for the partner is released after {waited_ms} ms: {reason}",
        id.describe()
    )
}

pub fn link_lost_line(held: usize, skip_asked: bool) -> String {
    let skip = if skip_asked {
        ", the asked skip is made here"
    } else {
        ""
    };
    format!("event: the partner is gone or not in play, {held} held plays are released{skip}")
}

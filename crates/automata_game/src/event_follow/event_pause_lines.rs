use crate::event_follow::{EventId, PARTNER_EVENT_PAUSE_HOLDS_AT_MOST_MS};

pub fn own_pause_told_line(id: EventId, paused: bool) -> String {
    let what = if paused { "opened" } else { "closed" };
    format!(
        "event: the event pause {what} here for {}, told the partner",
        id.describe()
    )
}

pub fn partner_pause_heard_line(id: EventId, paused: bool) -> String {
    let what = if paused { "opened" } else { "closed" };
    format!(
        "event: the partner's event pause for {} {what}",
        id.describe()
    )
}

pub fn partner_pause_ended_line(id: EventId) -> String {
    format!(
        "event: the partner's event pause for {} is dropped, another event plays here",
        id.describe()
    )
}

pub fn partner_pause_gave_up_line(id: EventId) -> String {
    format!(
        "event: the partner's event pause for {} lasted over {PARTNER_EVENT_PAUSE_HOLDS_AT_MOST_MS} ms, this copy goes on",
        id.describe()
    )
}

pub fn pause_clock_line(id: EventId) -> String {
    format!(
        "event: the event clock is paused here for the partner's pause of {}",
        id.describe()
    )
}

pub fn resume_clock_line() -> String {
    "event: the event clock is resumed here, the partner's pause is over".to_owned()
}

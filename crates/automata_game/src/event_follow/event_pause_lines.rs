use crate::event_follow::EventId;

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

pub fn partner_pause_event_ended_here_line(id: EventId) -> String {
    format!(
        "event: the partner's event pause for {} is dropped, the event ended here",
        id.describe()
    )
}

pub fn partner_pause_rule_ended_line(id: EventId) -> String {
    format!(
        "event: the partner's event pause for {} is dropped, the link is down or a copy is out of play",
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

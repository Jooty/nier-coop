use crate::event_follow::{EventId, Unattended};

pub fn own_skip_ask_line(id: EventId) -> String {
    format!(
        "event: the player asked to skip {}, held until the partner asks too",
        id.describe()
    )
}

pub fn vote_passed_line(id: EventId, runs_here: bool) -> String {
    let who = if runs_here {
        "the skip runs here"
    } else {
        "the host runs the skip"
    };
    format!("event: both players asked to skip {}, {who}", id.describe())
}

pub fn partner_skip_ask_heard_line(id: EventId) -> String {
    format!(
        "event: the partner asked to skip {}, shown on screen",
        id.describe()
    )
}

pub fn agreed_by_own_input_line(id: EventId) -> String {
    format!(
        "event: the player agreed to the partner's skip of {} with the skip input",
        id.describe()
    )
}

pub fn agreed_as_unattended_line(id: EventId, why: Unattended) -> String {
    format!(
        "event: this copy counts as agreeing to the partner's skip of {} because {}",
        id.describe(),
        why.label()
    )
}

pub fn partner_counted_as_agreeing_line(id: EventId) -> String {
    format!(
        "event: the partner counts as agreeing to skip {}: its state says it is unattended",
        id.describe()
    )
}

pub fn partner_does_not_count_line(id: EventId, why: &str) -> String {
    format!(
        "event: the partner does not count for the skip of {} ({why}), the skip runs here",
        id.describe()
    )
}

pub fn own_skip_ask_lapsed_line(id: EventId, waited_ms: u64) -> String {
    format!(
        "event: the ask to skip {} lapsed after {waited_ms} ms without the partner's, the event goes on",
        id.describe()
    )
}

pub fn partner_skip_ask_lapsed_line(id: EventId) -> String {
    format!("event: the partner's ask to skip {} lapsed", id.describe())
}

pub fn attendance_line(unattended: Option<Unattended>) -> String {
    match unattended {
        Some(why) => format!(
            "event: this copy is unattended for skip votes: {}",
            why.label()
        ),
        None => "event: this copy is attended again for skip votes".to_owned(),
    }
}

use std::time::Instant;

#[derive(Clone, Debug, Default)]
pub struct LinkTraffic {
    last_sent: Option<(String, Instant)>,
    last_received: Option<(String, Instant)>,
}

impl LinkTraffic {
    pub fn note_sent(&mut self, kind: &str, now: Instant) {
        self.last_sent = Some((kind.to_owned(), now));
    }

    pub fn note_received(&mut self, kind: &str, now: Instant) {
        self.last_received = Some((kind.to_owned(), now));
    }

    pub fn describe(&self, now: Instant) -> String {
        format!(
            "last sent {}; last received {}",
            describe_last(self.last_sent.as_ref(), now),
            describe_last(self.last_received.as_ref(), now)
        )
    }

    pub fn forget(&mut self) {
        self.last_sent = None;
        self.last_received = None;
    }
}

fn describe_last(last: Option<&(String, Instant)>, now: Instant) -> String {
    match last {
        None => "nothing".to_owned(),
        Some((kind, at)) => format!(
            "{kind} {} ms ago",
            now.saturating_duration_since(*at).as_millis()
        ),
    }
}

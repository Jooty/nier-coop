use crate::character::Character;
use crate::partner::PARTNER_DATA_WAIT_LIMIT_MS;

/// What the hook reads about the data of the partner's character while a load is held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartnerDataReading {
    pub wanted: Character,
    /// A spawn of the wanted character waits for nothing.
    pub loaded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartnerDataStep {
    /// The held load need not wait for the partner's data any longer.
    pub ready: bool,
    /// The game is to be asked, without waiting, to read this character's data.
    pub ask_for: Option<Character>,
    pub logs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Settled {
    NothingToWaitFor,
    Loaded(Character),
    GivenUp(Character),
}

/// Keeps a held load until the data of the partner's character is read, for a bounded time.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PartnerDataWait {
    waiting_since_ms: Option<u64>,
    asked_for: Option<Character>,
    ask_refused: bool,
    settled_logged: Option<Settled>,
}

impl PartnerDataWait {
    /// `reading` is `None` when no partner character can be told or its data cannot be asked for.
    pub fn step(&mut self, now_ms: u64, reading: Option<PartnerDataReading>) -> PartnerDataStep {
        let Some(reading) = reading else {
            return self.settle(
                Settled::NothingToWaitFor,
                "start: no partner character can be read ahead here, the held load does not wait for one"
                    .to_owned(),
            );
        };
        let wanted = reading.wanted;
        let name = wanted.launcher_id();
        if reading.loaded {
            let line = match self.waiting_since_ms {
                Some(since_ms) if self.asked_for == Some(wanted) => format!(
                    "start: the data of {name} is read, {} ms after it was asked for; the engine can make {name} as its partner",
                    now_ms.saturating_sub(since_ms)
                ),
                _ => format!(
                    "start: the data of {name} is loaded already; the engine can make {name} as its partner"
                ),
            };
            return self.settle(Settled::Loaded(wanted), line);
        }
        let since_ms = *self.waiting_since_ms.get_or_insert(now_ms);
        if self.ask_refused || now_ms.saturating_sub(since_ms) >= PARTNER_DATA_WAIT_LIMIT_MS {
            let why = if self.ask_refused {
                "the game refused to read it".to_owned()
            } else {
                format!(
                    "it was not read within {} s",
                    PARTNER_DATA_WAIT_LIMIT_MS / 1000
                )
            };
            return self.settle(
                Settled::GivenUp(wanted),
                format!(
                    "start: the data of {name} is not loaded ({why}); the load goes on, and the engine's own partner stands in until it is"
                ),
            );
        }
        let mut step = PartnerDataStep::default();
        if self.asked_for != Some(wanted) {
            self.asked_for = Some(wanted);
            step.ask_for = Some(wanted);
            step.logs.push(format!(
                "start: asking the game to read the data of {name} (object {:#X}) while the loading screen is held, at most {} s",
                wanted.object_id(),
                PARTNER_DATA_WAIT_LIMIT_MS / 1000
            ));
        }
        step
    }

    /// The game did not take the request asked for by the last step.
    pub fn ask_was_refused(&mut self) {
        self.ask_refused = true;
    }

    fn settle(&mut self, settled: Settled, line: String) -> PartnerDataStep {
        let logs = if self.settled_logged == Some(settled) {
            Vec::new()
        } else {
            self.settled_logged = Some(settled);
            vec![line]
        };
        PartnerDataStep {
            ready: true,
            ask_for: None,
            logs,
        }
    }
}

use crate::message_handlers::{MAX_RELAYED_HANDLER_PASSES, RELAYED_HANDLER_PASS_MS};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RelayedHandlerPasses {
    expected: Vec<(String, u64)>,
}

impl RelayedHandlerPasses {
    pub const fn new() -> Self {
        Self {
            expected: Vec::new(),
        }
    }

    pub fn clear(&mut self) {
        self.expected.clear();
    }

    pub fn expect_run(&mut self, name: &str, now_ms: u64) {
        self.prune(now_ms);
        if self.expected.len() >= MAX_RELAYED_HANDLER_PASSES {
            self.expected.remove(0);
        }
        self.expected.push((name.to_owned(), now_ms));
    }

    pub fn take(&mut self, name: &str, now_ms: u64) -> bool {
        self.prune(now_ms);
        let Some(index) = self.expected.iter().position(|(known, _)| known == name) else {
            return false;
        };
        self.expected.remove(index);
        true
    }

    fn prune(&mut self, now_ms: u64) {
        self.expected
            .retain(|(_, at)| now_ms.saturating_sub(*at) <= RELAYED_HANDLER_PASS_MS);
    }
}

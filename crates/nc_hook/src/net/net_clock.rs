use std::sync::OnceLock;
use std::time::Instant;

static NET_CLOCK_START: OnceLock<Instant> = OnceLock::new();

pub fn net_clock_ms() -> u64 {
    NET_CLOCK_START
        .get_or_init(Instant::now)
        .elapsed()
        .as_millis() as u64
}

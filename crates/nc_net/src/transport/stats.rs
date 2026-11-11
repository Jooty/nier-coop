use std::time::Duration;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LinkStats {
    pub rtt: Option<Duration>,
    pub direct: bool,
    pub lost_packets: u64,
}

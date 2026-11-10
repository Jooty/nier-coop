use std::time::Duration;

mod config;
mod invite_codec;
mod link;
mod transport;

pub use config::IrohConfig;
pub use transport::IrohTransport;

const ALPN: &[u8] = b"rc-net/1";
const INVITE_PREFIX: &str = "rcn1";
const ONLINE_WAIT_SECONDS: u64 = 15;
const CLOSE_WAIT_SECONDS: u64 = 3;
const WORKER_THREADS: usize = 2;
const PROTOCOL_ERROR_CODE: u32 = 1;
const BYE_CODE: u32 = 0;
const DIRECT_ADDRESS_POLLS: u32 = 100;
const DIRECT_ADDRESS_POLL_INTERVAL: Duration = Duration::from_millis(50);

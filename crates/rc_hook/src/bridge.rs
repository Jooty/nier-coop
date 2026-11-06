mod commands;
mod net;
mod phases;
mod puppets;
mod server;
mod snapshot;

use std::net::Ipv4Addr;

pub(crate) use commands::error_reply;
pub use server::start;
pub(crate) use snapshot::field;

const BRIDGE_PORT: u16 = 47100;
const BRIDGE_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;
const MAX_READ_LEN: usize = 4096;
const MAX_REQUEST_LINE_BYTES: u64 = 8192;
const DEFAULT_LOG_LINES: usize = 50;
const MAX_LOG_LINES: usize = 500;

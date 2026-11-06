mod commands;
mod server;
mod snapshot;

use std::net::Ipv4Addr;

pub use server::start;

const BRIDGE_PORT: u16 = 47100;
const BRIDGE_BIND_ADDRESS: Ipv4Addr = Ipv4Addr::LOCALHOST;
const MAX_READ_LEN: usize = 4096;
const MAX_REQUEST_LINE_BYTES: u64 = 8192;
const DEFAULT_LOG_LINES: usize = 50;
const MAX_LOG_LINES: usize = 500;

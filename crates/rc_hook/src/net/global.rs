use std::sync::OnceLock;

use rc_game::build::SUPPORTED_PE_IDENTITY;
use rc_net::LocalLinkTransport;
use rc_proto::BuildGate;

use crate::diagnostics::log_line;
use crate::net::NetThread;

static NET_THREAD: OnceLock<NetThread> = OnceLock::new();

pub fn handle() -> Option<&'static NetThread> {
    NET_THREAD.get()
}

pub fn start_link(port: u16) {
    let build = BuildGate {
        time_date_stamp: SUPPORTED_PE_IDENTITY.time_date_stamp,
        size_of_image: SUPPORTED_PE_IDENTITY.size_of_image,
    };
    match NetThread::spawn(LocalLinkTransport::game(build), port) {
        Ok(thread) => {
            let _ = NET_THREAD.set(thread);
        }
        Err(error) => log_line(&format!("net: cannot start the net thread ({error})")),
    }
}

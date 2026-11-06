use std::sync::OnceLock;

use rc_game::build::SUPPORTED_PE_IDENTITY;
use rc_net::{IrohConfig, IrohTransport};
use rc_proto::BuildGate;

use crate::diagnostics::log_line;
use crate::net::{DEFAULT_PLAYER_NAME, NetThread};

static PLAYER_NAME: OnceLock<String> = OnceLock::new();
static NET_THREAD: OnceLock<Option<NetThread>> = OnceLock::new();

pub fn set_player_name(name: String) {
    let _ = PLAYER_NAME.set(name);
}

pub fn handle() -> Option<&'static NetThread> {
    NET_THREAD.get_or_init(spawn_net_thread).as_ref()
}

fn spawn_net_thread() -> Option<NetThread> {
    let name = PLAYER_NAME
        .get()
        .cloned()
        .unwrap_or_else(|| DEFAULT_PLAYER_NAME.to_owned());
    let build = BuildGate {
        time_date_stamp: SUPPORTED_PE_IDENTITY.time_date_stamp,
        size_of_image: SUPPORTED_PE_IDENTITY.size_of_image,
    };
    let spawned = NetThread::spawn(|| IrohTransport::new(IrohConfig::default()), name, build);
    match spawned {
        Ok(thread) => Some(thread),
        Err(error) => {
            log_line(&format!("net: cannot start the net thread ({error})"));
            None
        }
    }
}

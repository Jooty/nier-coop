use std::sync::OnceLock;

use automata_game::build::{COMPAT_PE_IDENTITY, MAIN_PE_IDENTITY};
use automata_game::character::Character;
use automata_game::exe::ExeVariant;
use automata_proto::BuildGate;
use nc_hook::diagnostics::log_line;
use nc_hook::menu::CoOpStatus;
use nc_hook::net::{LinkPeer, NetThread};
use nc_net::LocalLinkTransport;

use crate::net::automata_net::AutomataNet;
use crate::net::co_op_view::{co_op_status_of, link_not_started_status};

static NET_THREAD: OnceLock<NetThread<AutomataNet>> = OnceLock::new();
static PARTNER_SETTING: OnceLock<Character> = OnceLock::new();

pub fn handle() -> Option<&'static NetThread<AutomataNet>> {
    NET_THREAD.get()
}

pub fn start_link(port: u16, variant: ExeVariant, partner_setting: Character) {
    let identity = match variant {
        ExeVariant::Main => MAIN_PE_IDENTITY,
        ExeVariant::Compat => COMPAT_PE_IDENTITY,
    };
    let build = BuildGate {
        time_date_stamp: identity.time_date_stamp,
        size_of_image: identity.size_of_image,
    };
    let _ = PARTNER_SETTING.set(partner_setting);
    let spawned = NetThread::spawn(
        AutomataNet::new(partner_setting),
        LocalLinkTransport::game(build),
        LinkPeer::launcher_on_port(port),
    );
    match spawned {
        Ok(thread) => {
            let _ = NET_THREAD.set(thread);
            log_line(&format!(
                "net: net worker started, launcher link on port {port}"
            ));
        }
        Err(error) => log_line(&format!("net: cannot start the net thread ({error})")),
    }
}

pub fn co_op_status() -> CoOpStatus {
    match (NET_THREAD.get(), PARTNER_SETTING.get()) {
        (Some(net), Some(partner_setting)) => co_op_status_of(&net.view(), *partner_setting),
        _ => link_not_started_status(),
    }
}

use automata_game::travel::{TeleportRequest, TravelPoint, TravelView, decide_bring_partner};
use automata_proto::Message;
use nc_hook::diagnostics::log_line;
use nc_hook::net::NetState;

use crate::actions::{Action, enqueue};
use crate::menu::TeleportHooks;
use crate::net::handle;
use crate::travel_driver::driver_state::VIEW;

const GO_TO_PARTNER_QUEUED_TEXT: &str = "asked the game to move you to the partner";
const BRING_PARTNER_SENT_TEXT: &str = "asked the other player's game to bring them here";

pub fn teleport_hooks() -> TeleportHooks {
    TeleportHooks {
        view,
        ask_jump,
        ask_go_to_partner,
        ask_bring_partner,
    }
}

fn view() -> TravelView {
    VIEW.lock()
        .ok()
        .and_then(|view| view.clone())
        .unwrap_or_default()
}

fn ask_jump(point: TravelPoint) -> Result<(), String> {
    enqueue(Action::TravelTo(point)).map_err(|error| error.to_string())
}

fn ask_go_to_partner() -> Result<String, String> {
    enqueue(Action::GoToPartner)
        .map(|()| GO_TO_PARTNER_QUEUED_TEXT.to_owned())
        .map_err(|error| format!("not queued: {error}"))
}

fn ask_bring_partner() -> Result<String, String> {
    let net = handle();
    let connected = net.is_some_and(|net| net.update(|shared| shared.state == NetState::Connected));
    decide_bring_partner(connected).map_err(|refusal| format!("not asked: {refusal}"))?;
    if let Some(net) = net {
        net.send_relayed(Message::Teleport(TeleportRequest::ComeToSender));
    }
    log_line("teleport: asked the other player's game to bring them here (a reliable message)");
    Ok(BRING_PARTNER_SENT_TEXT.to_owned())
}

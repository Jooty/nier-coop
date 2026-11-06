use std::sync::{Mutex, OnceLock, PoisonError};

use rc_game::puppet::BodyAnnouncer;
use rc_proto::{Message, PartnerBody};

use crate::diagnostics::log_line;
use crate::net::{self, AutoConfig, net_clock_ms};

static LOCAL_BODY: OnceLock<PartnerBody> = OnceLock::new();
static REMOTE_BODY: Mutex<Option<PartnerBody>> = Mutex::new(None);
static ANNOUNCER: Mutex<BodyAnnouncer> = Mutex::new(BodyAnnouncer::new());

pub(super) fn local_body() -> PartnerBody {
    *LOCAL_BODY.get_or_init(|| {
        let body = AutoConfig::load_beside_game()
            .ok()
            .flatten()
            .and_then(|config| config.body)
            .unwrap_or_default();
        log_line(&format!("body: this game wears {body:?} for the partner"));
        body
    })
}

pub(super) fn exchange_bodies() -> Option<PartnerBody> {
    let net = net::handle()?;
    let announce = ANNOUNCER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_due(local_body(), net_clock_ms());
    if announce {
        net.send_relayed(Message::PartnerBody(local_body()));
    }
    let arrived = net.take_relayed(|message| matches!(message, Message::PartnerBody(_)));
    let mut remote = REMOTE_BODY.lock().unwrap_or_else(PoisonError::into_inner);
    for message in arrived {
        if let Message::PartnerBody(body) = message
            && *remote != Some(body)
        {
            log_line(&format!("body: the partner wears {body:?}"));
            *remote = Some(body);
        }
    }
    *remote
}

pub(super) fn forget_remote_body() {
    *REMOTE_BODY.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

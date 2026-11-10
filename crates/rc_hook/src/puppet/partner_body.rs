use std::sync::{Mutex, PoisonError};

use rc_game::effects::local_has_book;
use rc_game::puppet::{BodyAnnouncer, BookAnnouncer, body_for_pack, body_info};
use rc_proto::{BodyPackName, Message, PartnerBody};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::memory::InProcessMemory;
use crate::net::{self, net_clock_ms};
use crate::player_body::real_body;

static REMOTE_BODY: Mutex<Option<PartnerBody>> = Mutex::new(None);
static REMOTE_HAS_BOOK: Mutex<bool> = Mutex::new(false);
static ANNOUNCER: Mutex<BodyAnnouncer> = Mutex::new(BodyAnnouncer::new());
static BOOK_ANNOUNCER: Mutex<BookAnnouncer> = Mutex::new(BookAnnouncer::new());

pub(super) fn exchange_bodies() -> Option<PartnerBody> {
    let net = net::handle()?;
    if let Some(real) = real_body() {
        let announce = ANNOUNCER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_due(real.body, real.phase_loads, net_clock_ms());
        if announce && let Ok(pack) = BodyPackName::new(body_info(real.body).pack_name) {
            net.send_relayed(Message::PartnerBody(pack));
        }
    }
    let arrived = net.take_relayed(|message| matches!(message, Message::PartnerBody(_)));
    let mut remote = REMOTE_BODY.lock().unwrap_or_else(PoisonError::into_inner);
    for message in arrived {
        let Message::PartnerBody(pack) = message else {
            continue;
        };
        let Some(body) = body_for_pack(pack.as_str()) else {
            log_line(&format!(
                "body: the partner wears pack {}, which this build has no entry for",
                pack.as_str()
            ));
            continue;
        };
        if *remote != Some(body) {
            log_line(&format!(
                "body: the partner wears {body:?} (pack {})",
                pack.as_str()
            ));
            if body == PartnerBody::SeedKaineE {
                log_line(
                    "kaine: the partner plays as her; the puppet wears her own pack KAINE015 \
                     (requested if missing, about 290 MB) and follows her motions by name",
                );
            }
            *remote = Some(body);
        }
    }
    *remote
}

pub(super) fn exchange_book_presence(build: &VerifiedBuild) -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let game = rc_game::Replicant::new(InProcessMemory::new(build));
    if let Ok(has_book) = local_has_book(game.memory()) {
        let due = BOOK_ANNOUNCER
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .is_due(has_book, net_clock_ms());
        if due {
            net.send_relayed(Message::PartnerBook(has_book));
        }
    }
    let arrived = net.take_relayed(|message| matches!(message, Message::PartnerBook(_)));
    let mut remote = REMOTE_HAS_BOOK
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    for message in arrived {
        let Message::PartnerBook(has_book) = message else {
            continue;
        };
        if *remote != has_book {
            log_line(&format!("book: the partner has a book now: {has_book}"));
            *remote = has_book;
        }
    }
    *remote
}

pub(super) fn forget_remote_body() {
    *REMOTE_HAS_BOOK
        .lock()
        .unwrap_or_else(PoisonError::into_inner) = false;
    *REMOTE_BODY.lock().unwrap_or_else(PoisonError::into_inner) = None;
}

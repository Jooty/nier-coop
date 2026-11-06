use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use rc_game::new_game::{TitleHold, TitleHoldInputs, title_hold};

use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::net::{self, NetState};

static HOLD_STARTED: OnceLock<Instant> = OnceLock::new();
static HOLD_ANNOUNCED: AtomicBool = AtomicBool::new(false);

pub(super) fn hold_for_partner() -> bool {
    let Some(net) = net::handle() else {
        return false;
    };
    let started = *HOLD_STARTED.get_or_init(Instant::now);
    let view = net.view();
    let waited_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let decision = title_hold(TitleHoldInputs {
        waited_ms,
        launcher_linked: view.state != NetState::WaitingForLauncher,
        partner_connected: view.state == NetState::Connected,
        partner_readiness: view.partner.readiness(),
    });
    match decision {
        TitleHold::Hold => {
            if !HOLD_ANNOUNCED.swap(true, Ordering::Relaxed) {
                log_line("boot: title held until the partner's game is open");
                let partner = view
                    .peer_name
                    .as_deref()
                    .map_or_else(|| "your partner".to_owned(), str::to_owned);
                post_event(&format!("Waiting for {partner}'s game"));
            }
            true
        }
        TitleHold::Release => {
            if HOLD_ANNOUNCED.load(Ordering::Relaxed) {
                log_line(&format!("boot: title released after {waited_ms} ms"));
            }
            false
        }
        TitleHold::TimedOut => {
            log_line(&format!(
                "boot: title hold timed out after {waited_ms} ms, starting alone"
            ));
            false
        }
    }
}

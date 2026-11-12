use std::sync::atomic::{AtomicBool, Ordering};

use automata_game::mode::SessionMode;
use automata_proto::PlayerState;
use nc_hook::diagnostics::log_line;
use nc_hook::net::{NetState, Player};

use crate::net::link_state::NetShared;
use crate::net::partner_watch::PartnerWatch;

static UNKNOWN_MODE_LOGGED: AtomicBool = AtomicBool::new(false);

pub trait SessionModeExt {
    fn session_mode(&self) -> SessionMode;

    /// The question every campaign-only module asks: the partner, and only in the campaign.
    fn campaign_partner(&self) -> Option<&Player<PlayerState, PartnerWatch>>;

    /// The link is up and the session is the campaign: the other copy takes part in the story.
    fn campaign_link_is_up(&self) -> bool;

    /// The player whose body the companion slot carries: the partner in the campaign, the
    /// lowest other slot in free roam.
    fn companion_player(&self) -> Option<&Player<PlayerState, PartnerWatch>>;
}

impl SessionModeExt for NetShared {
    fn session_mode(&self) -> SessionMode {
        if !SessionMode::is_a_known_mode(self.rules.mode)
            && !UNKNOWN_MODE_LOGGED.swap(true, Ordering::Relaxed)
        {
            log_line(&format!(
                "session: mode {} is not known to this game, treated as free roam",
                self.rules.mode
            ));
        }
        SessionMode::from_rules(self.rules)
    }

    fn campaign_partner(&self) -> Option<&Player<PlayerState, PartnerWatch>> {
        self.session_mode()
            .story_rules_apply()
            .then(|| self.partner())
            .flatten()
    }

    fn campaign_link_is_up(&self) -> bool {
        self.state == NetState::Connected && self.session_mode().story_rules_apply()
    }

    fn companion_player(&self) -> Option<&Player<PlayerState, PartnerWatch>> {
        if self.session_mode().story_rules_apply() {
            self.partner()
        } else {
            self.others().next()
        }
    }
}

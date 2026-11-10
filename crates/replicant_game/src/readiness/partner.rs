use replicant_proto::GameReadiness;

use crate::readiness::{JOINING_IGNORED_AFTER_MS, SILENT_PARTNER_FALLBACK_MS};

pub const PARTNER_GAME_CLOSED_LABEL: &str = "disconnected: their game closed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PartnerWatch {
    readiness: GameReadiness,
    connected_at_ms: Option<u64>,
    heard_from_partner: bool,
    game_closed: bool,
}

impl PartnerWatch {
    pub const fn new() -> Self {
        Self {
            readiness: GameReadiness::NotLinked,
            connected_at_ms: None,
            heard_from_partner: false,
            game_closed: false,
        }
    }

    pub fn partner_connected(&mut self, now_ms: u64) {
        self.readiness = GameReadiness::NotLinked;
        self.connected_at_ms = Some(now_ms);
        self.heard_from_partner = false;
        self.game_closed = false;
    }

    pub fn partner_sent_state(&mut self) -> bool {
        if self.connected_at_ms.is_none() || self.readiness.is_in_play() {
            return false;
        }
        self.readiness = GameReadiness::InPlay;
        self.heard_from_partner = true;
        self.game_closed = false;
        true
    }

    pub fn assume_in_play_if_silent(&mut self, now_ms: u64) -> bool {
        let Some(connected_at) = self.connected_at_ms else {
            return false;
        };
        if self.heard_from_partner
            || now_ms.saturating_sub(connected_at) < SILENT_PARTNER_FALLBACK_MS
        {
            return false;
        }
        self.readiness = GameReadiness::InPlay;
        self.heard_from_partner = true;
        true
    }

    pub fn partner_gone(&mut self) {
        *self = Self::new();
    }

    pub fn partner_reported(&mut self, readiness: GameReadiness) -> bool {
        if self.connected_at_ms.is_none() {
            return false;
        }
        let just_closed = readiness == GameReadiness::NotLinked
            && self.readiness != GameReadiness::NotLinked
            && !self.game_closed;
        self.game_closed =
            readiness == GameReadiness::NotLinked && (self.game_closed || just_closed);
        self.readiness = readiness;
        self.heard_from_partner = true;
        just_closed
    }

    pub fn is_game_closed(self) -> bool {
        self.connected_at_ms.is_some() && self.game_closed
    }

    pub fn readiness(self) -> GameReadiness {
        self.readiness
    }

    pub fn is_in_play(self) -> bool {
        self.connected_at_ms.is_some() && self.readiness.is_in_play()
    }

    pub fn is_joining(self) -> bool {
        self.connected_at_ms.is_some() && !self.readiness.is_in_play()
    }

    pub fn counts_for_zone_rule(self, now_ms: u64) -> bool {
        let Some(connected_at) = self.connected_at_ms else {
            return false;
        };
        if self.game_closed {
            return false;
        }
        self.readiness.is_in_play()
            || now_ms.saturating_sub(connected_at) < JOINING_IGNORED_AFTER_MS
    }
}

pub fn joining_label(readiness: GameReadiness) -> &'static str {
    match readiness {
        GameReadiness::NotLinked => "joining: their game is not open yet",
        GameReadiness::Loading => "joining: their game is on the title screen or loading",
        GameReadiness::InPlay => "in play",
    }
}

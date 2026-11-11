use crate::downed::{
    DOWNED_HEALTH, DOWNED_POSE_STATE, DownedEffect, FallBackCause, GotUpCause, REVIVE_HOLD_SECONDS,
    REVIVE_RANGE_UNITS,
};

/// The log line of one step of going down and reviving.
pub fn effect_log_line(effect: DownedEffect) -> String {
    match effect {
        DownedEffect::WentDown => format!(
            "downed: the hero went down instead of being defeated (health held at {DOWNED_HEALTH}); it is put in the blown-down state {DOWNED_POSE_STATE:#X} and kept there, no hit reaches its health, and the other player can revive it"
        ),
        DownedEffect::GotUp(GotUpCause::Revived) => {
            "downed: revived by the other player; the hero is let out of the pose state and gets a quarter of its health".to_owned()
        }
        DownedEffect::GotUp(GotUpCause::PartnerGone) => {
            "downed: the other player is gone or cannot revive any more, the hero gets up by itself with a quarter of its health".to_owned()
        }
        DownedEffect::FellBack(cause) => format!(
            "downed: fell back to the game's own defeat ({}); the hero is let out of the pose state and its defeat runs as the game runs it",
            fall_back_cause_text(cause)
        ),
        DownedEffect::DefeatedWhileUp => {
            "downed: the game defeated the hero while it was up (its defeat was not held off, or came by a way the hold does not cover); the game's own defeat runs and the other player is told nobody revives until it is over".to_owned()
        }
        DownedEffect::BackFromTheGamesDefeat => {
            "downed: the game's own defeat is over, the hero is up again".to_owned()
        }
        DownedEffect::BeingRevived(percent) => {
            format!("downed: the other player's revive of the hero is at {percent}%")
        }
        DownedEffect::PartnerWentDown => format!(
            "downed: the other player went down; standing within {REVIVE_RANGE_UNITS} units for {REVIVE_HOLD_SECONDS} s revives it"
        ),
        DownedEffect::PartnerGotUp => "downed: the other player is up again".to_owned(),
        DownedEffect::PartnerFellBack => {
            "downed: the other player fell back to its game's own defeat".to_owned()
        }
        DownedEffect::ReviveStarted => {
            "downed: revive started, the hero stands close to the other player".to_owned()
        }
        DownedEffect::ReviveProgress(percent) => format!("downed: revive at {percent}%"),
        DownedEffect::ReviveBroken => {
            "downed: revive broken off before it was done, it starts from nothing again".to_owned()
        }
        DownedEffect::ReviveDone => {
            "downed: revive done; the other player is told until it is up".to_owned()
        }
    }
}

fn fall_back_cause_text(cause: FallBackCause) -> &'static str {
    match cause {
        FallBackCause::BothDown => "both players are down",
        FallBackCause::PartnerDefeated => "the other player was defeated by its game",
        FallBackCause::GaveUp => "the player gave up waiting",
        FallBackCause::DefeatedByTheGame => {
            "the game defeated the hero while it was down, by a way the hold does not cover"
        }
    }
}

use crate::downed::{DownedEffect, GotUpCause};

pub fn downed_effect_line(effect: DownedEffect, partner_name: &str) -> Option<String> {
    match effect {
        DownedEffect::WentDown => Some("You were downed".to_owned()),
        DownedEffect::GotUp(GotUpCause::Revived) => Some(format!("{partner_name} revived you")),
        DownedEffect::PartnerWentDown => Some(format!("{partner_name} was downed")),
        DownedEffect::ReviveDone => Some(format!("You revived {partner_name}")),
        DownedEffect::FellBack(_) => Some("Nobody could revive you".to_owned()),
        _ => None,
    }
}

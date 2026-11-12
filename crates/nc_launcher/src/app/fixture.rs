use crate::app::{Launcher, Screen};
use crate::config::LauncherConfig;
use crate::home::GameCard;
use crate::net::LinkStatus;
use nc_launcher_core::GameProfile;

pub fn fixture_launcher(
    profile: &'static GameProfile,
    cards: Vec<GameCard>,
    link: LinkStatus,
    screen: Screen,
) -> Launcher {
    let mut launcher = Launcher::from_parts(LauncherConfig::default(), None);
    launcher.profile = profile;
    let active = cards
        .iter()
        .find(|card| card.profile.game_id == launcher.profile.game_id);
    launcher.game = active.and_then(|card| card.install.clone());
    launcher.dll_source = active.and_then(|card| card.dll_source.clone());
    launcher.mod_status = active.map_or(launcher.mod_status, |card| card.mod_status);
    launcher.cards = cards;
    launcher.link = link;
    launcher.screen = screen;
    launcher
}

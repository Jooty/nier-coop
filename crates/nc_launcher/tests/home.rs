use std::path::PathBuf;

use nc_launcher::home::{CardTone, GameCard, card_view};
use nc_launcher::install::ModStatus;
use nc_launcher::steam::GameInstall;
use nc_launcher_core::{Character, GameProfile, GameSupport};

use crate::scratch::fresh_dir;

const MADE_UP_CHARACTERS: [Character; 1] = [Character {
    id: "only",
    label: "Only",
}];

const SUPPORTED_GAME: GameProfile = GameProfile {
    game_id: "made-up",
    display_name: "Made Up",
    steam_app_id: "424242",
    install_folder_name: "MadeUp",
    exe_names: &["MadeUp.exe"],
    loader_dll_name: "loader.dll",
    hook_dll_name: "made_up_hook.dll",
    link_file_name: "made-up.toml",
    characters: &MADE_UP_CHARACTERS,
    characters_shareable: false,
    modes: &[nc_launcher_core::CO_OP_CAMPAIGN],
    support: GameSupport {
        install: true,
        start: true,
        saves: true,
    },
};

const UNSUPPORTED_GAME: GameProfile = GameProfile {
    support: GameSupport {
        install: false,
        start: false,
        saves: false,
    },
    ..SUPPORTED_GAME
};

fn card(
    profile: &'static GameProfile,
    install: Option<GameInstall>,
    status: ModStatus,
) -> GameCard {
    GameCard {
        profile,
        install,
        dll_source: None,
        mod_status: status,
        installed_build: Some("abcd1234".to_owned()),
    }
}

fn install_with_exe(profile: &'static GameProfile, name: &str, with_exe: bool) -> GameInstall {
    let game_dir = fresh_dir(name);
    if with_exe {
        std::fs::write(game_dir.join(profile.exe_names[0]), b"x").unwrap();
    }
    GameInstall {
        profile,
        steamapps_dir: PathBuf::from("/lib/steamapps"),
        game_dir,
        proton_prefix: None,
    }
}

#[test]
fn a_game_that_is_not_found_is_greyed_out_and_cannot_be_played() {
    let view = card_view(&card(&SUPPORTED_GAME, None, ModStatus::NotInstalled));
    assert_eq!(view.found_text, "not found");
    assert_eq!(view.found_tone, CardTone::Bad);
    assert!(view.greyed_out);
    assert!(!view.play_enabled);
    assert!(!view.offers_install);
}

#[test]
fn a_folder_without_the_exe_counts_as_not_found() {
    let install = install_with_exe(&SUPPORTED_GAME, "home_no_exe", false);
    let view = card_view(&card(
        &SUPPORTED_GAME,
        Some(install),
        ModStatus::NotInstalled,
    ));
    assert!(
        view.found_text
            .starts_with("not found: no game executable in")
    );
    assert!(view.greyed_out && !view.play_enabled);
}

#[test]
fn a_found_game_shows_where_and_can_be_played() {
    let install = install_with_exe(&SUPPORTED_GAME, "home_found", true);
    let where_text = install.game_dir.display().to_string();
    let view = card_view(&card(
        &SUPPORTED_GAME,
        Some(install),
        ModStatus::NotInstalled,
    ));
    assert_eq!(view.title, "Made Up");
    assert_eq!(view.found_text, format!("found: {where_text}"));
    assert_eq!(view.found_tone, CardTone::Good);
    assert!(!view.greyed_out && view.play_enabled && view.offers_install);
    assert_eq!(view.mod_text, "mod not installed");
}

#[test]
fn the_mod_line_follows_the_install_state_and_names_the_build() {
    let install = install_with_exe(&SUPPORTED_GAME, "home_mod_states", true);
    let line = |status| card_view(&card(&SUPPORTED_GAME, Some(install.clone()), status)).mod_text;
    assert_eq!(
        line(ModStatus::Current),
        "mod installed, current build (abcd1234)"
    );
    assert_eq!(
        line(ModStatus::Outdated),
        "mod installed, older build (abcd1234)"
    );
    assert_eq!(
        line(ModStatus::Foreign),
        "a different loader.dll is in the game folder"
    );
}

#[test]
fn a_profile_that_supports_nothing_says_the_mod_is_not_built_and_offers_no_install() {
    let install = install_with_exe(&UNSUPPORTED_GAME, "home_unsupported", true);
    let view = card_view(&card(
        &UNSUPPORTED_GAME,
        Some(install),
        ModStatus::NotInstalled,
    ));
    assert_eq!(view.mod_text, "mod not built yet");
    assert_eq!(view.mod_tone, CardTone::Muted);
    assert!(!view.offers_install);
    assert!(view.play_enabled);
}

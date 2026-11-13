use nc_launcher::game_link::{StartChoice, StartSetting};
use nc_launcher::lobby::launch_start;
use nc_launcher::net::{LOBBY_MODE_CONTINUE, LOBBY_MODE_NEW_GAME};

fn configured() -> StartSetting {
    StartSetting {
        choice: StartChoice::Slot,
        index: 3,
        slot: 2,
        story_allowed: false,
    }
}

#[test]
fn a_game_whose_hook_runs_the_campaign_starts_the_lobbys_new_game() {
    let start = launch_start(configured(), true, LOBBY_MODE_NEW_GAME, true);
    assert_eq!(start.choice, StartChoice::NewGame);
    assert_eq!(start.slot, 0);
    assert!(start.story_allowed);
}

#[test]
fn a_game_whose_hook_runs_the_campaign_continues_on_the_continue_page() {
    let start = launch_start(configured(), true, LOBBY_MODE_CONTINUE, true);
    assert_eq!(start.choice, StartChoice::Continue);
    assert!(start.story_allowed);
}

#[test]
fn free_roam_and_the_other_game_keep_the_configured_start() {
    assert_eq!(
        launch_start(configured(), true, LOBBY_MODE_NEW_GAME, false),
        configured()
    );
    assert_eq!(
        launch_start(configured(), false, LOBBY_MODE_CONTINUE, true),
        configured()
    );
}

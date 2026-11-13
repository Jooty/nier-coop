use replicant_game::fade_sync::{CoverLift, SceneCover, SceneCovers};

const MAIN_LAYER: u64 = 0x1_44B1_C860;
const EVENT_LAYER: u64 = 0x1_44B1_C6E0;
const END_COVERED_MS: u64 = 51_374_974;
const UNPLAYED_END: &str = "EID_1410_a0060_End";

fn covered_by_the_unplayed_end() -> SceneCovers {
    let mut covers = SceneCovers::new();
    assert!(covers.note_cover(MAIN_LAYER, UNPLAYED_END, END_COVERED_MS, true, false));
    covers
}

fn end_cover() -> SceneCover {
    SceneCover {
        layer: MAIN_LAYER,
        covered_ms: END_COVERED_MS,
        by: UNPLAYED_END.to_owned(),
    }
}

#[test]
fn the_cover_of_an_unplayed_scene_is_lifted_when_the_follow_gives_up() {
    let mut covers = covered_by_the_unplayed_end();
    assert_eq!(covers.step(true, false, false), None);
    assert_eq!(
        covers.step(true, true, false),
        Some((CoverLift::FollowGaveUp, vec![end_cover()]))
    );
    assert!(covers.is_empty());
    assert_eq!(covers.step(false, false, false), None);
}

#[test]
fn the_cover_is_lifted_when_the_hosts_scene_ends_first() {
    let mut covers = covered_by_the_unplayed_end();
    assert_eq!(covers.step(true, false, false), None);
    assert_eq!(
        covers.step(false, false, false),
        Some((CoverLift::HostSceneEnded, vec![end_cover()]))
    );
}

#[test]
fn a_cover_while_this_side_plays_the_scene_or_no_host_scene_runs_is_not_kept() {
    let mut covers = SceneCovers::new();
    assert!(!covers.note_cover(MAIN_LAYER, UNPLAYED_END, END_COVERED_MS, true, true));
    assert!(!covers.note_cover(MAIN_LAYER, UNPLAYED_END, END_COVERED_MS, false, false));
    assert!(covers.is_empty());
}

#[test]
fn the_games_own_reveal_of_that_layer_forgets_the_cover() {
    let mut covers = covered_by_the_unplayed_end();
    assert!(covers.note_cover(EVENT_LAYER, UNPLAYED_END, END_COVERED_MS, true, false));
    covers.note_reveal(MAIN_LAYER);
    assert_eq!(
        covers
            .step(true, true, false)
            .map(|(_, lifted)| lifted.len()),
        Some(1)
    );
}

#[test]
fn entering_the_scene_hands_the_screen_to_the_scene() {
    let mut covers = covered_by_the_unplayed_end();
    assert_eq!(covers.step(true, false, true), None);
    assert_eq!(covers.step(true, true, false), None);
}

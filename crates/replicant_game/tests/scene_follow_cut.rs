use replicant_game::scene_follow::{
    EVENT_CUT_LOAD_FUNCTION, EVENT_CUT_NAME_BUFFER_BYTES, LocalCut,
    UNPLAYED_END_AFTER_START_MAX_MS, UnplayedStarts, cut_load_call, cut_name_from_buffer,
    cut_to_load, host_scene_cut,
};
use replicant_proto::{SceneCut, StoryCall, StoryCallArg};

const ENTRANCE_EVENT: &str = "EID_1410_a0060";
const ENTRANCE_AUDIO: &str = "MA_EID_1410_a0060";
const ENTRANCE_START: &str = "EID_1410_a0060_Start";
const ENTRANCE_END: &str = "EID_1410_a0060_End";
const GUEST_START_POPPED_MS: u64 = 51_374_958;
const GUEST_END_POPPED_MS: u64 = 51_374_974;

fn buffer(text: &str) -> [u8; EVENT_CUT_NAME_BUFFER_BYTES] {
    let mut bytes = [0_u8; EVENT_CUT_NAME_BUFFER_BYTES];
    bytes[..text.len()].copy_from_slice(text.as_bytes());
    bytes
}

fn entrance_cut() -> SceneCut {
    SceneCut {
        event: ENTRANCE_EVENT.to_owned(),
        audio: ENTRANCE_AUDIO.to_owned(),
    }
}

fn guest_after_the_failed_start() -> UnplayedStarts {
    let mut starts = UnplayedStarts::new();
    starts.note_popped(ENTRANCE_START, GUEST_START_POPPED_MS);
    starts.note_no_cut_reveal(GUEST_START_POPPED_MS);
    starts
}

#[test]
fn the_host_sends_the_cut_its_event_manager_holds() {
    assert_eq!(
        host_scene_cut(true, &buffer(ENTRANCE_EVENT), &buffer(ENTRANCE_AUDIO)),
        Some(entrance_cut())
    );
    assert_eq!(
        host_scene_cut(false, &buffer(ENTRANCE_EVENT), &buffer(ENTRANCE_AUDIO)),
        None
    );
    assert_eq!(
        host_scene_cut(true, &buffer(""), &buffer(ENTRANCE_AUDIO)),
        None
    );
    assert_eq!(
        host_scene_cut(
            true,
            &buffer(ENTRANCE_EVENT),
            &[0xFF; EVENT_CUT_NAME_BUFFER_BYTES]
        ),
        Some(SceneCut {
            event: ENTRANCE_EVENT.to_owned(),
            audio: String::new(),
        })
    );
}

#[test]
fn a_name_buffer_without_its_end_is_not_a_name() {
    assert_eq!(
        cut_name_from_buffer(&[b'E'; EVENT_CUT_NAME_BUFFER_BYTES]),
        None
    );
    assert_eq!(
        cut_name_from_buffer(&buffer(ENTRANCE_EVENT)).as_deref(),
        Some(ENTRANCE_EVENT)
    );
}

#[test]
fn a_guest_that_skipped_the_preloading_fight_start_loads_the_hosts_cut() {
    let nothing_loaded = LocalCut {
        loaded: false,
        event: None,
    };
    let other_cut = LocalCut {
        loaded: true,
        event: Some("EID_1410_a0040g".to_owned()),
    };
    let same_cut = LocalCut {
        loaded: true,
        event: Some(ENTRANCE_EVENT.to_owned()),
    };
    assert_eq!(
        cut_to_load(&nothing_loaded, Some(&entrance_cut())),
        Some(entrance_cut())
    );
    assert_eq!(
        cut_to_load(&other_cut, Some(&entrance_cut())),
        Some(entrance_cut())
    );
    assert_eq!(cut_to_load(&same_cut, Some(&entrance_cut())), None);
    assert_eq!(cut_to_load(&nothing_loaded, None), None);
}

#[test]
fn the_cut_is_loaded_through_the_scripts_own_preload_call() {
    assert_eq!(
        cut_load_call(25, &entrance_cut()),
        StoryCall {
            phase: 25,
            pack: 0,
            function: EVENT_CUT_LOAD_FUNCTION.to_owned(),
            args: vec![
                StoryCallArg::Text(ENTRANCE_EVENT.as_bytes().to_vec()),
                StoryCallArg::Text(ENTRANCE_AUDIO.as_bytes().to_vec()),
            ],
        }
    );
    let no_audio = SceneCut {
        event: ENTRANCE_EVENT.to_owned(),
        audio: String::new(),
    };
    assert_eq!(
        cut_load_call(25, &no_audio).args[1],
        StoryCallArg::Number((-1.0_f64).to_bits())
    );
}

#[test]
fn the_end_that_popped_16_ms_after_its_start_with_no_cut_loaded_ended_an_unplayed_scene() {
    let mut starts = guest_after_the_failed_start();
    assert_eq!(
        starts.take_unplayed_start(ENTRANCE_END, false, GUEST_END_POPPED_MS),
        Some(ENTRANCE_START.to_owned())
    );
    assert_eq!(
        starts.take_unplayed_start(ENTRANCE_END, false, GUEST_END_POPPED_MS + 1),
        None
    );
}

#[test]
fn a_natural_end_with_its_cut_still_loaded_runs() {
    let mut starts = guest_after_the_failed_start();
    assert_eq!(
        starts.take_unplayed_start(ENTRANCE_END, true, GUEST_END_POPPED_MS),
        None
    );
}

#[test]
fn an_end_a_start_calls_itself_without_the_engines_reveal_runs() {
    let mut starts = UnplayedStarts::new();
    starts.note_popped("EID_1410_a0050g_Start", GUEST_START_POPPED_MS);
    assert_eq!(
        starts.take_unplayed_start("EID_1410_a0050g_End", false, GUEST_END_POPPED_MS),
        None
    );
}

#[test]
fn another_scenes_end_or_a_late_end_runs() {
    let mut starts = guest_after_the_failed_start();
    assert_eq!(
        starts.take_unplayed_start("EID_1410_a0040g_End", false, GUEST_END_POPPED_MS),
        None
    );
    assert_eq!(
        starts.take_unplayed_start(
            ENTRANCE_END,
            false,
            GUEST_START_POPPED_MS + UNPLAYED_END_AFTER_START_MAX_MS + 1
        ),
        None
    );
}

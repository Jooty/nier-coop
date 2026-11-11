use std::collections::BTreeSet;

use replicant_game::fight_sync::{
    BASE_PLAYER_LOCAL_HANDLERS, FIGHT_WINDOWS, FightWindow, FightWindowData, GuestFightStream,
    GuestPop, GuestReceived, HostFightStream, HostStreamOut, PopSource, StreamEntry, WindowArming,
    WindowCloseCause, WindowEvent, fight_windows_of_pack,
};
use replicant_proto::{Message, decode, encode};

const DOCS_COPY: &str = include_str!("../data/fight-sync.toml");
const CLIFF_VILLAGE: &str = "A_CLIFF_VILLAGE_01";
const CLIFF_VILLAGE_PHASE: u16 = 40;
const HERE: Option<u16> = Some(CLIFF_VILLAGE_PHASE);
const SECOND_STORY_FIGHT_START: &str = "EID_1050_a0020_Start";
const SECOND_STORY_FIGHT_HAND_OFF: &str = "EID_1050_a0070_Start";
const SOUND_TRAP_IN: &str = "PLAY_SE_WOODUP01";
const SOUND_TRAP_OUT: &str = "STOP_SE_WOODUP01";
const SHARED_SIDE_VIEW_TRAP_WITH_A_PUT: &str = "SIDEVIEW01_01";
const SECOND_STORY_FIGHT_LOCAL_TRAPS: [&str; 13] = [
    "PLAY_SE_WOODUP01",
    "PLAY_SE_WOODUP02",
    "PLAY_SE_WOODUP03",
    "PLAY_SE_WOODUP04",
    "STOP_SE_WOODUP01",
    "STOP_SE_WOODUP02",
    "STOP_SE_WOODUP03",
    "STOP_SE_WOODUP04",
    "DEFAULT_SHADOW_DISTANCE",
    "LONG_SHADOW_DISTANCE",
    "Set_Bright",
    "Change_Bright_07",
    "Change_Bright_04",
];
const SECOND_STORY_FIGHT_CAMERA_CLUSTER: [&str; 41] = [
    "CB_Free_Camera_01",
    "CB_Free_Camera_02",
    "CB_Free_Camera_03",
    "CB_Free_Camera_04",
    "CB_Free_Camera_05",
    "CB_Free_Camera_06",
    "CB_Free_Camera_07",
    "CB_Free_Camera_31",
    "CB_Free_Camera_32",
    "CB_Free_Camera_33",
    "CB_Free_Camera_34",
    "CB_Free_Camera_35",
    "CB_Free_Camera_36",
    "SET_SIDEVIEW_TRAP",
    "SET_SIDEVIEW_TRAP2",
    "SET_SIDEVIEW_TRAP3",
    "SIDEFRONT_EID1050_01",
    "SIDEFRONT_EID1050_02",
    "SIDEFRONT_EID1050_03",
    "SIDEVIEW02_01",
    "SIDEVIEW02_02",
    "SIDEVIEW02_03",
    "SIDEVIEW02_04",
    "SIDEVIEW03_01",
    "SIDEVIEW03_02",
    "SIDEVIEW04_01",
    "SIDEVIEWOUT_01",
    "SIDEVIEWOUT_02",
    "SIDEVIEWOUT_EID1050",
    "SIDEVIEW_EID1050CA03_01",
    "SIDEVIEW_EID1050CA03_02",
    "SIDEVIEW_EID1050CA03_03",
    "SIDEVIEW_EID1050CA03_04",
    "SIDEVIEW_FRONT01",
    "SIDEVIEW_FRONT02",
    "SIDEVIEW_FRONT03",
    "SIDEVIEW_FRONT04",
    "SIDEVIEW_FRONT05",
    "SIDEVIEW_FRONT06",
    "snow_Foot_Efect_On",
    "snow_Foot_Efect_Off",
];
const SECOND_STORY_FIGHT_SIDE_VIEW_TRAPS_WITH_A_PUT: [&str; 2] = ["SIDEVIEW01_01", "SIDEVIEW05_01"];
const SECOND_STORY_FIGHT_FALL_TRAPS: [&str; 2] = ["FALL_TRAP_01_04", "FALL_TRAP_01_04_01"];
const STONE_FIELD: &str = "A_STONE_FIELD_02";
const STONE_FIELD_ARM_IDS: [&str; 2] = ["EID_0200_a0030g_Talk00", "EID_0200_a0030g_Talk01"];
const STONE_FIELD_END: &str = "EID_0200_a0080_End";
const STONE_FIELD_SURVEY_LOCAL_COUNT: usize = 24;

struct LocalTrap {
    pack: String,
    name: String,
    natives: Vec<String>,
}

fn docs() -> toml::Table {
    toml::from_str(DOCS_COPY).expect("the docs copy of fight-sync.toml parses")
}

fn strs(value: Option<&toml::Value>) -> Vec<String> {
    value
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|v| v.as_str().expect("a name").to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn local_traps() -> Vec<LocalTrap> {
    docs()["player_local_trap"]
        .as_array()
        .expect("player-local trap rows")
        .iter()
        .map(|row| LocalTrap {
            pack: row["pack"].as_str().expect("a pack").to_owned(),
            name: row["name"].as_str().expect("a name").to_owned(),
            natives: strs(row.get("natives")),
        })
        .collect()
}

fn second_story_fight() -> &'static FightWindowData {
    FIGHT_WINDOWS
        .iter()
        .find(|row| (row.number, row.part) == (8, 4))
        .expect("window 8-4")
}

fn is_player_local(row: &FightWindowData, name: &str) -> bool {
    row.player_local.contains(&name) || BASE_PLAYER_LOCAL_HANDLERS.contains(&name)
}

fn over_the_wire(message: Message) -> Message {
    assert_eq!(message.validate(), Ok(()));
    decode(&encode(&message).unwrap()).unwrap()
}

fn open_second_story_fight() -> (HostFightStream, Vec<HostStreamOut>) {
    let mut host = HostFightStream::new();
    host.enter_phase(CLIFF_VILLAGE_PHASE, CLIFF_VILLAGE);
    let armed = host.popped(SECOND_STORY_FIGHT_START, Vec::new(), false);
    assert!(
        matches!(
            armed.as_slice(),
            [HostStreamOut::Armed {
                by: SECOND_STORY_FIGHT_START,
                arming: WindowArming::StartId,
                ..
            }]
        ),
        "{armed:?}"
    );
    let opened = host.callback_registered();
    assert!(
        matches!(
            opened.as_slice(),
            [
                HostStreamOut::Opened { window, armed_by: Some(SECOND_STORY_FIGHT_START), .. },
                HostStreamOut::Entry(StreamEntry { seq: 0, name, .. })
            ] if *window == second_story_fight() && name == SECOND_STORY_FIGHT_START
        ),
        "{opened:?}"
    );
    (host, opened)
}

fn entries(outs: &[HostStreamOut]) -> Vec<&str> {
    outs.iter()
        .filter_map(|out| match out {
            HostStreamOut::Entry(StreamEntry { name, .. }) => Some(name.as_str()),
            _ => None,
        })
        .collect()
}

fn joined_guest() -> GuestFightStream {
    let (_host, opened) = open_second_story_fight();
    let mut guest = GuestFightStream::new();
    let message = opened
        .iter()
        .find_map(HostStreamOut::message)
        .expect("the open message");
    let received = guest.receive_message(over_the_wire(message), HERE);
    assert!(matches!(received, GuestReceived::Opened { .. }));
    guest
}

#[test]
fn no_player_local_trap_reaches_a_native_outside_the_per_player_lists() {
    let doc = docs();
    let allowed: BTreeSet<String> = [
        "per_player_natives",
        "per_player_removal_natives",
        "per_player_registration_natives",
        "per_player_flag_read_natives",
        "per_player_flag_write_natives",
    ]
    .iter()
    .flat_map(|key| strs(doc.get(*key)))
    .collect();
    let traps = local_traps();
    assert!(!traps.is_empty());
    for trap in &traps {
        for native in &trap.natives {
            assert!(
                allowed.contains(native),
                "{} in {} reaches {native}",
                trap.name,
                trap.pack
            );
        }
    }
}

#[test]
fn every_window_runs_its_packs_player_local_traps_on_each_game() {
    let traps = local_traps();
    for row in &FIGHT_WINDOWS {
        for trap in traps.iter().filter(|trap| trap.pack == row.pack) {
            assert!(
                row.player_local.contains(&trap.name.as_str()),
                "{} missing from fight {}-{}",
                trap.name,
                row.number,
                row.part
            );
        }
    }
}

#[test]
fn no_arm_id_is_player_local() {
    for row in &FIGHT_WINDOWS {
        for name in row.arm_ids {
            assert!(
                !is_player_local(row, name),
                "arm id {name} of fight {}-{} is player-local",
                row.number,
                row.part
            );
        }
    }
}

#[test]
fn no_start_or_close_id_is_a_player_local_trap_so_the_window_rules_never_need_one() {
    let traps = local_traps();
    for row in &FIGHT_WINDOWS {
        let ids = [
            row.start_ids,
            row.end_ids,
            row.change_map_ids,
            row.game_over_ids,
            row.continue_ids,
        ];
        for name in ids.iter().flat_map(|ids| ids.iter()) {
            assert!(
                !traps
                    .iter()
                    .any(|trap| trap.pack == row.pack && trap.name == *name),
                "{name} of fight {}-{} is a player-local trap",
                row.number,
                row.part
            );
        }
    }
}

#[test]
fn the_second_story_fights_sound_shadow_and_brightness_traps_are_player_local() {
    let window = second_story_fight();
    for name in SECOND_STORY_FIGHT_LOCAL_TRAPS {
        assert!(window.player_local.contains(&name), "{name}");
    }
}

#[test]
fn the_second_story_fights_camera_and_side_view_traps_without_a_put_are_player_local() {
    let window = second_story_fight();
    for name in SECOND_STORY_FIGHT_CAMERA_CLUSTER {
        assert!(window.player_local.contains(&name), "{name}");
    }
    for name in SECOND_STORY_FIGHT_SIDE_VIEW_TRAPS_WITH_A_PUT {
        assert!(!is_player_local(window, name), "{name}");
        assert!(window.window_functions.contains(&name), "{name}");
    }
}

#[test]
fn the_second_story_fights_fall_traps_stay_shared() {
    let window = second_story_fight();
    for name in SECOND_STORY_FIGHT_FALL_TRAPS {
        assert!(!is_player_local(window, name), "{name}");
    }
}

#[test]
fn the_second_story_fight_no_longer_arms_on_a_sound_trap() {
    let window = second_story_fight();
    assert!(window.arm_ids.is_empty(), "{:?}", window.arm_ids);
    assert!(window.start_ids.contains(&SECOND_STORY_FIGHT_START));
    assert!(!window.start_ids.contains(&SOUND_TRAP_OUT));
    let mut fight = FightWindow::new(CLIFF_VILLAGE);
    assert_eq!(fight.function_ran(SOUND_TRAP_OUT), None);
    assert_eq!(fight.function_ran(SOUND_TRAP_IN), None);
}

#[test]
fn the_second_story_fight_closes_when_it_hands_off_to_the_next_event() {
    let window = second_story_fight();
    assert!(window.end_ids.contains(&SECOND_STORY_FIGHT_HAND_OFF));
    let mut fight = FightWindow::new(CLIFF_VILLAGE);
    assert_eq!(
        fight.function_ran(SECOND_STORY_FIGHT_START),
        Some(WindowEvent::Armed(window, SECOND_STORY_FIGHT_START))
    );
    assert_eq!(
        fight.callback_registered(),
        Some(WindowEvent::Opened(window))
    );
    assert_eq!(
        fight.function_ran(SECOND_STORY_FIGHT_HAND_OFF),
        Some(WindowEvent::Closed(window, WindowCloseCause::End))
    );
}

#[test]
fn the_first_boss_fight_keeps_its_arm_ids_end_and_survey_locals() {
    let window = fight_windows_of_pack(STONE_FIELD).next().expect("window 1");
    assert_eq!(window.arm_ids, STONE_FIELD_ARM_IDS);
    assert_eq!(window.end_ids, [STONE_FIELD_END]);
    assert_eq!(window.player_local.len(), STONE_FIELD_SURVEY_LOCAL_COUNT);
    assert!(local_traps().iter().all(|trap| trap.pack != STONE_FIELD));
}

#[test]
fn the_hosts_own_player_local_trap_pop_is_not_streamed_and_a_shared_one_is() {
    let (mut host, _opened) = open_second_story_fight();
    assert_eq!(
        entries(&host.popped(SOUND_TRAP_IN, Vec::new(), false)),
        Vec::<&str>::new()
    );
    assert_eq!(
        entries(&host.popped(SOUND_TRAP_OUT, Vec::new(), false)),
        Vec::<&str>::new()
    );
    assert_eq!(
        entries(&host.popped(SHARED_SIDE_VIEW_TRAP_WITH_A_PUT, Vec::new(), false)),
        [SHARED_SIDE_VIEW_TRAP_WITH_A_PUT]
    );
}

#[test]
fn the_host_still_closes_on_the_hand_off_after_player_local_pops() {
    let (mut host, _opened) = open_second_story_fight();
    host.popped(SOUND_TRAP_IN, Vec::new(), false);
    host.popped(SOUND_TRAP_OUT, Vec::new(), false);
    let outs = host.popped(SECOND_STORY_FIGHT_HAND_OFF, Vec::new(), false);
    assert!(
        outs.iter().any(|out| matches!(
            out,
            HostStreamOut::Closed {
                cause: WindowCloseCause::End,
                ..
            }
        )),
        "{outs:?}"
    );
}

#[test]
fn the_guests_own_player_local_trap_runs_at_once_and_a_shared_one_is_held() {
    let mut guest = joined_guest();
    assert_eq!(
        guest.pop_by_player(SOUND_TRAP_IN, Vec::new(), HERE, false, 0),
        GuestPop::RunPlayerLocal
    );
    assert_eq!(
        guest.pop(SOUND_TRAP_OUT, PopSource::Fired, 0, HERE, false),
        GuestPop::RunPlayerLocal
    );
    assert_eq!(
        guest.pop_by_player(SHARED_SIDE_VIEW_TRAP_WITH_A_PUT, Vec::new(), HERE, false, 0),
        GuestPop::HeldForHostCopy
    );
}

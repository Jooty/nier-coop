use rc_game::fight_sync::{
    BASE_PLAYER_LOCAL_HANDLERS, BASE_PLAYER_LOCAL_MESSAGES, FIGHT_WINDOWS, FightWindow,
    FightWindowData, REQUEST_REGISTRATION_NATIVES, TAPED_NATIVES, WindowCloseCause, WindowEvent,
    WindowState, fight_windows_of_pack, is_request_registration_native, is_taped_native,
};

const DOCS_COPY: &str = include_str!("../data/fight-sync.toml");
const STONE_FIELD: &str = "A_STONE_FIELD_02";
const STONE_FIELD_START: &str = "EID_0200_a0030g_Start";
const STONE_FIELD_END: &str = "EID_0200_a0080_End";
const STONE_FIELD_GAME_OVER: &str = "LIB_GameOver";
const STONE_FIELD_CONTINUE: &str = "EID_0200_a0040g_Continue";
const STONE_FIELD_TALK: &str = "EID_0200_a0030g_Talk02";
const DESERT_TEMPLE: &str = "A_DESERT_TEMPLE_01";
const DESERT_TEMPLE_START: &str = "EID_1450_a0070_End";
const DESERT_TEMPLE_MAP_CHANGE: &str = "JUMP_MAP_02";
const PLANNER_TAPED_NATIVES: [&str; 10] = [
    "_GetRandI",
    "_GetRandF",
    "_GetActorLife",
    "_GetActorLifeRate",
    "_GetUniqueActorMaxLife",
    "_GetActorCount",
    "_GetActorCountGroups",
    "_GetActorNumInScriptArea",
    "_GetNearPlayerEnemyNum",
    "_GetGlobalTimeCount",
];

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

fn ints(value: Option<&toml::Value>) -> Vec<i64> {
    value
        .and_then(toml::Value::as_array)
        .map(|items| {
            items
                .iter()
                .map(|v| v.as_integer().expect("a number"))
                .collect()
        })
        .unwrap_or_default()
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_owned()).collect()
}

fn stone_field() -> &'static FightWindowData {
    fight_windows_of_pack(STONE_FIELD)
        .next()
        .expect("fight 1 window")
}

fn armed_and_open(pack: &str, start: &str) -> FightWindow {
    let mut window = FightWindow::new(pack);
    assert!(matches!(
        window.function_ran(start),
        Some(WindowEvent::Armed(_))
    ));
    assert!(matches!(
        window.callback_registered(),
        Some(WindowEvent::Opened(_))
    ));
    window
}

#[test]
fn the_table_has_every_window_of_the_docs_copy_with_the_same_ids() {
    let doc = docs();
    let rows = doc["fight"].as_array().expect("fight rows");
    assert_eq!(rows.len(), FIGHT_WINDOWS.len());
    for (row, table) in rows.iter().zip(FIGHT_WINDOWS.iter()) {
        let at = |key: &str| row.get(key);
        assert_eq!(
            at("number").and_then(toml::Value::as_integer),
            Some(i64::from(table.number))
        );
        assert_eq!(
            at("part").and_then(toml::Value::as_integer),
            Some(i64::from(table.part))
        );
        assert_eq!(at("pack").and_then(toml::Value::as_str), Some(table.pack));
        assert_eq!(strs(at("start_ids")), owned(table.start_ids));
        assert_eq!(strs(at("end_ids")), owned(table.end_ids));
        assert_eq!(strs(at("change_map_ids")), owned(table.change_map_ids));
        assert_eq!(strs(at("game_over_ids")), owned(table.game_over_ids));
        assert_eq!(strs(at("continue_ids")), owned(table.continue_ids));
        assert_eq!(strs(at("player_local")), owned(table.player_local));
        assert_eq!(strs(at("run_at_once")), owned(table.run_at_once));
        assert_eq!(strs(at("window_functions")), owned(table.window_functions));
        assert_eq!(strs(at("taped_functions")), owned(table.taped_functions));
        let messages: Vec<i64> = table
            .player_local_messages
            .iter()
            .map(|&m| m.into())
            .collect();
        assert_eq!(ints(at("player_local_messages")), messages);
        let ids: Vec<i64> = table
            .player_local_callback_ids
            .iter()
            .map(|&i| i.into())
            .collect();
        assert_eq!(ints(at("player_local_callback_ids")), ids);
    }
}

#[test]
fn the_shared_lists_match_the_docs_copy() {
    let doc = docs();
    assert_eq!(strs(doc.get("taped_natives")), owned(&TAPED_NATIVES));
    assert_eq!(
        strs(doc.get("request_registration_natives")),
        owned(&REQUEST_REGISTRATION_NATIVES)
    );
    assert_eq!(
        strs(doc.get("base_player_local")),
        owned(&BASE_PLAYER_LOCAL_HANDLERS)
    );
    let messages: Vec<i64> = BASE_PLAYER_LOCAL_MESSAGES
        .iter()
        .map(|&m| m.into())
        .collect();
    assert_eq!(ints(doc.get("base_player_local_messages")), messages);
}

#[test]
fn every_window_has_a_start_id_and_a_way_to_close() {
    for row in &FIGHT_WINDOWS {
        assert!(
            !row.start_ids.is_empty(),
            "fight {}-{}",
            row.number,
            row.part
        );
        assert!(
            !(row.end_ids.is_empty()
                && row.change_map_ids.is_empty()
                && row.game_over_ids.is_empty()
                && row.continue_ids.is_empty()),
            "fight {}-{} never closes",
            row.number,
            row.part
        );
    }
}

#[test]
fn the_tape_holds_the_planners_ten_state_reads_and_nothing_else() {
    assert_eq!(TAPED_NATIVES, PLANNER_TAPED_NATIVES);
    assert!(is_taped_native("_GetGlobalTimeCount"));
    assert!(!is_taped_native("_GetPlayerPosition"));
}

#[test]
fn a_player_move_callback_is_a_request() {
    assert!(is_request_registration_native("_SetPlayerMoveCallback"));
    assert!(!is_request_registration_native("_SetPadInputCallback"));
}

#[test]
fn a_registration_before_any_start_id_opens_nothing() {
    let mut window = FightWindow::new(STONE_FIELD);
    assert_eq!(window.callback_registered(), None);
    assert_eq!(window.function_ran(STONE_FIELD_TALK), None);
    assert_eq!(window.state(), WindowState::Closed);
}

#[test]
fn the_window_opens_on_the_first_registration_after_the_start_id() {
    let mut window = FightWindow::new(STONE_FIELD);
    assert_eq!(
        window.function_ran(STONE_FIELD_START),
        Some(WindowEvent::Armed(stone_field()))
    );
    assert_eq!(window.open_window(), None);
    assert_eq!(
        window.callback_registered(),
        Some(WindowEvent::Opened(stone_field()))
    );
    assert_eq!(window.callback_registered(), None);
    assert_eq!(window.open_window(), Some(stone_field()));
}

#[test]
fn the_end_id_closes_the_window() {
    let mut window = armed_and_open(STONE_FIELD, STONE_FIELD_START);
    assert_eq!(
        window.function_ran(STONE_FIELD_END),
        Some(WindowEvent::Closed(stone_field(), WindowCloseCause::End))
    );
    assert_eq!(window.state(), WindowState::Closed);
}

#[test]
fn a_game_over_closes_the_window() {
    let mut window = armed_and_open(STONE_FIELD, STONE_FIELD_START);
    assert_eq!(
        window.function_ran(STONE_FIELD_GAME_OVER),
        Some(WindowEvent::Closed(
            stone_field(),
            WindowCloseCause::GameOver
        ))
    );
}

#[test]
fn a_continue_closes_the_window_and_the_start_id_arms_it_again() {
    let mut window = armed_and_open(STONE_FIELD, STONE_FIELD_START);
    assert_eq!(
        window.function_ran(STONE_FIELD_CONTINUE),
        Some(WindowEvent::Closed(
            stone_field(),
            WindowCloseCause::Continue
        ))
    );
    assert_eq!(
        window.function_ran(STONE_FIELD_START),
        Some(WindowEvent::Armed(stone_field()))
    );
    assert_eq!(
        window.callback_registered(),
        Some(WindowEvent::Opened(stone_field()))
    );
}

#[test]
fn a_map_change_closes_the_window() {
    let row = fight_windows_of_pack(DESERT_TEMPLE)
        .next()
        .expect("fight 2 window");
    let mut window = armed_and_open(DESERT_TEMPLE, DESERT_TEMPLE_START);
    assert_eq!(
        window.function_ran(DESERT_TEMPLE_MAP_CHANGE),
        Some(WindowEvent::Closed(row, WindowCloseCause::ChangeMap))
    );
}

#[test]
fn leaving_the_pack_closes_the_window() {
    let mut window = armed_and_open(STONE_FIELD, STONE_FIELD_START);
    assert_eq!(window.enter_pack(STONE_FIELD), None);
    assert_eq!(
        window.enter_pack(DESERT_TEMPLE),
        Some(WindowEvent::Closed(
            stone_field(),
            WindowCloseCause::PhaseLeft
        ))
    );
    assert_eq!(window.pack(), DESERT_TEMPLE);
    assert_eq!(window.enter_pack(STONE_FIELD), None);
}

#[test]
fn a_close_id_while_armed_disarms_the_window() {
    let mut window = FightWindow::new(STONE_FIELD);
    window.function_ran(STONE_FIELD_START);
    assert_eq!(
        window.function_ran(STONE_FIELD_END),
        Some(WindowEvent::Closed(stone_field(), WindowCloseCause::End))
    );
    assert_eq!(window.callback_registered(), None);
}

#[test]
fn a_start_id_inside_an_open_window_is_part_of_the_fight() {
    let mut window = armed_and_open(STONE_FIELD, STONE_FIELD_START);
    assert_eq!(window.function_ran(STONE_FIELD_START), None);
    assert_eq!(window.function_ran(STONE_FIELD_TALK), None);
    assert_eq!(window.open_window(), Some(stone_field()));
}

#[test]
fn another_packs_start_id_does_not_arm_the_window() {
    let mut window = FightWindow::new(STONE_FIELD);
    assert_eq!(window.function_ran(DESERT_TEMPLE_START), None);
    assert_eq!(window.state(), WindowState::Closed);
}

#[test]
fn player_local_names_join_the_base_list_and_the_windows_additions() {
    let row = stone_field();
    assert!(row.is_player_local_handler("SQ_Fishing_Radio"));
    assert!(row.is_player_local_handler("B_Lance_Talk"));
    assert!(!row.is_player_local_handler(STONE_FIELD_TALK));
    assert!(row.is_player_local_message(210));
    assert!(row.is_player_local_message(213));
    assert!(!row.is_player_local_message(212));
    assert!(row.is_player_local_callback_id(5115));
    assert!(!row.is_player_local_callback_id(5116));
}

#[test]
fn every_taped_function_is_a_window_function() {
    for window in &FIGHT_WINDOWS {
        for name in window.taped_functions {
            assert!(
                window.window_functions.contains(name),
                "{name} in fight {}-{}",
                window.number,
                window.part
            );
        }
    }
}

#[test]
fn only_taped_or_unknown_entries_wait_for_the_hosts_end() {
    let window = stone_field();
    assert!(window.entry_waits_for_host_end("CB_BossCA_01"));
    assert!(!window.entry_waits_for_host_end("EID_0200_a0030g_End"));
    assert!(!window.entry_waits_for_host_end("AttackCount_Talk"));
    assert!(window.entry_waits_for_host_end("Not_A_Function_Of_This_Window"));
}

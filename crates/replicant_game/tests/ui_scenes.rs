use replicant_game::ui_scenes::{
    LARGE_MAP_SCENE_ID, PAUSE_MENU_SCENE_ID, SHORT_SYSTEM_MENU_SCENE_ID, UI_LINK_NEXT_OFFSET,
    UI_MANAGER_FIRST_SCENE_LINK_OFFSET, UI_MANAGER_RVA, read_open_ui_scenes,
};

use crate::fake_memory::{FAKE_MODULE_BASE, FAKE_UI_SCENE_HEAP, FakeMemory};

const TITLE_MENU_SCENE_ID: u8 = 0;
const SHOP_MENU_SCENE_ID: u8 = 9;

fn scenes(ids: &[u8], short_open: bool) -> replicant_game::ui_scenes::OpenUiScenes {
    let mut memory = FakeMemory::new();
    memory.put_ui_scenes(ids, short_open);
    read_open_ui_scenes(&memory).unwrap()
}

#[test]
fn a_ui_manager_the_game_has_not_built_reads_as_no_scene_open() {
    let mut memory = FakeMemory::new();
    let manager = FAKE_MODULE_BASE + u64::from(UI_MANAGER_RVA);
    memory.put(
        manager + UI_MANAGER_FIRST_SCENE_LINK_OFFSET,
        &0_u64.to_le_bytes(),
    );
    let open = read_open_ui_scenes(&memory).unwrap();
    assert!(open.scene_ids().is_empty());
    assert!(!open.pause_menu_open());
}

#[test]
fn an_empty_scene_list_has_no_pause_menu() {
    assert!(!scenes(&[], false).pause_menu_open());
}

#[test]
fn the_pause_menu_or_the_map_scene_means_the_pause_menu_is_open() {
    let open = scenes(&[SHORT_SYSTEM_MENU_SCENE_ID, PAUSE_MENU_SCENE_ID], false);
    assert_eq!(
        open.scene_ids(),
        vec![SHORT_SYSTEM_MENU_SCENE_ID, PAUSE_MENU_SCENE_ID]
    );
    assert!(open.pause_menu_open());
    assert!(scenes(&[LARGE_MAP_SCENE_ID], false).pause_menu_open());
}

#[test]
fn the_resident_short_system_menu_counts_only_while_its_open_flag_is_set() {
    assert!(!scenes(&[SHORT_SYSTEM_MENU_SCENE_ID], false).pause_menu_open());
    let open = scenes(&[SHORT_SYSTEM_MENU_SCENE_ID], true);
    assert!(open.short_system_menu_open);
    assert!(open.pause_menu_open());
}

#[test]
fn a_shop_or_the_title_menu_is_listed_but_is_not_the_pause_menu() {
    let open = scenes(&[TITLE_MENU_SCENE_ID, SHOP_MENU_SCENE_ID], false);
    assert!(open.contains(SHOP_MENU_SCENE_ID));
    assert!(!open.pause_menu_open());
}

#[test]
fn a_scene_ring_that_never_closes_stops_at_the_walk_limit() {
    let mut memory = FakeMemory::new();
    memory.put_ui_scenes(&[SHOP_MENU_SCENE_ID], false);
    memory.put(
        FAKE_UI_SCENE_HEAP + UI_LINK_NEXT_OFFSET,
        &FAKE_UI_SCENE_HEAP.to_le_bytes(),
    );
    let open = read_open_ui_scenes(&memory).unwrap();
    assert_eq!(open.scene_ids(), vec![SHOP_MENU_SCENE_ID]);
}

use rc_game::offsets::{KEY_ITEM_FIRST_ID, KEY_ITEM_LAST_ID};
use rc_game::pickups::{is_key_item, item_name, local_key_item_notice, plan_partner_pickup};
use rc_proto::ItemPickup;

const MATERIAL_ID: u16 = 0x8A;
const KEY_ITEM_ID: u16 = 0x100;

fn pickup(item_id: u16, count: u8) -> ItemPickup {
    ItemPickup { item_id, count }
}

#[test]
fn item_ids_are_named_from_the_inventory_table() {
    assert_eq!(item_name(MATERIAL_ID), Some("Gold Ore"));
    assert_eq!(item_name(0), Some("Medicinal Herb"));
    assert_eq!(item_name(0x2FF), None);
}

#[test]
fn key_items_are_exactly_the_key_id_range() {
    assert!(!is_key_item(KEY_ITEM_FIRST_ID - 1));
    assert!(is_key_item(KEY_ITEM_FIRST_ID));
    assert!(is_key_item(KEY_ITEM_LAST_ID));
    assert!(!is_key_item(KEY_ITEM_LAST_ID + 1));
}

#[test]
fn a_partner_material_pickup_is_only_logged() {
    let plan = plan_partner_pickup("Ann", pickup(MATERIAL_ID, 1));
    assert_eq!(plan.notice, "Ann picked up Gold Ore");
    assert_eq!(plan.grant, None);
}

#[test]
fn a_partner_stack_pickup_shows_the_count() {
    let plan = plan_partner_pickup("Ann", pickup(MATERIAL_ID, 3));
    assert_eq!(plan.notice, "Ann picked up Gold Ore x3");
}

#[test]
fn a_partner_key_item_is_logged_as_one_and_granted_to_us() {
    let plan = plan_partner_pickup("Ann", pickup(KEY_ITEM_ID, 1));
    assert_eq!(plan.notice, "Ann picked up Moon Key (key item)");
    assert_eq!(plan.grant, Some(pickup(KEY_ITEM_ID, 1)));
}

#[test]
fn an_unnamed_id_is_logged_by_number() {
    let plan = plan_partner_pickup("Ann", pickup(0x2FF, 1));
    assert_eq!(plan.notice, "Ann picked up item 767");
}

#[test]
fn our_own_key_item_pickup_says_it_is_shared_and_a_material_says_nothing() {
    assert_eq!(
        local_key_item_notice("Ann", pickup(KEY_ITEM_ID, 1)).as_deref(),
        Some("Moon Key is a key item, Ann gets it too")
    );
    assert_eq!(local_key_item_notice("Ann", pickup(MATERIAL_ID, 1)), None);
}

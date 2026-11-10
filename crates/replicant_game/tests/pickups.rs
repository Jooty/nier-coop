use replicant_game::offsets::{KEY_ITEM_FIRST_ID, KEY_ITEM_LAST_ID};
use replicant_game::pickups::{
    classify_add_caller, is_key_item, is_story_item, item_name, local_key_item_notice,
    plan_partner_pickup, story_item_raise_amount,
};
use replicant_proto::{ItemPickup, PickupSource};

const MATERIAL_ID: u16 = 0x8A;
const KEY_ITEM_ID: u16 = 0x100;

fn pickup(item_id: u16, count: u8) -> ItemPickup {
    ItemPickup {
        item_id,
        count,
        source: PickupSource::World,
    }
}

fn pickup_from(item_id: u16, source: PickupSource) -> ItemPickup {
    ItemPickup {
        item_id,
        count: 1,
        source,
    }
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

#[test]
fn a_shop_purchase_is_logged_as_bought_and_a_script_reward_as_got() {
    let bought = plan_partner_pickup("Ann", pickup_from(MATERIAL_ID, PickupSource::Shop));
    let got = plan_partner_pickup("Ann", pickup_from(MATERIAL_ID, PickupSource::Other));
    assert_eq!(bought.notice, "Ann bought Gold Ore");
    assert_eq!(got.notice, "Ann got Gold Ore");
    assert_eq!(bought.grant, None);
}

#[test]
fn a_key_item_keeps_its_wording_whatever_the_source() {
    let plan = plan_partner_pickup("Ann", pickup_from(KEY_ITEM_ID, PickupSource::Shop));
    assert_eq!(plan.notice, "Ann picked up Moon Key (key item)");
}

#[test]
fn the_caller_decides_how_an_add_is_told() {
    assert_eq!(classify_add_caller(0x6C_52E0), PickupSource::World);
    assert_eq!(classify_add_caller(0x4C_26E7), PickupSource::Shop);
    assert_eq!(classify_add_caller(0x6E_41E8), PickupSource::Other);
    assert_eq!(classify_add_caller(0x3A_891E), PickupSource::Other);
    assert_eq!(classify_add_caller(0), PickupSource::Other);
}

#[test]
fn story_items_are_key_items_and_ordinary_ones_are_not() {
    assert!(is_story_item(723));
    assert!(is_key_item(723));
    assert!(!is_story_item(167));
    assert!(!is_key_item(167));
    assert!(!is_story_item(522));
    assert!(is_story_item(512));
    assert!(is_story_item(727));
    assert!(!is_story_item(728));
}

#[test]
fn a_story_item_is_raised_to_the_senders_count_and_never_above() {
    assert_eq!(story_item_raise_amount(1, 0), 1);
    assert_eq!(story_item_raise_amount(2, 1), 1);
    assert_eq!(story_item_raise_amount(1, 1), 0);
    assert_eq!(story_item_raise_amount(1, 3), 0);
}

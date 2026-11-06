use rc_proto::ItemPickup;

use crate::pickups::{is_key_item, item_name};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartnerPickupPlan {
    pub notice: String,
    pub grant: Option<ItemPickup>,
}

pub fn plan_partner_pickup(partner_name: &str, pickup: ItemPickup) -> PartnerPickupPlan {
    let item = describe_item(pickup);
    if is_key_item(pickup.item_id) {
        PartnerPickupPlan {
            notice: format!("{partner_name} picked up {item} (key item)"),
            grant: Some(pickup),
        }
    } else {
        PartnerPickupPlan {
            notice: format!("{partner_name} picked up {item}"),
            grant: None,
        }
    }
}

pub fn local_key_item_notice(partner_name: &str, pickup: ItemPickup) -> Option<String> {
    is_key_item(pickup.item_id).then(|| {
        format!(
            "{} is a key item, {partner_name} gets it too",
            describe_item(pickup)
        )
    })
}

fn describe_item(pickup: ItemPickup) -> String {
    let name = item_name(pickup.item_id).map_or_else(
        || format!("item {}", pickup.item_id),
        std::borrow::ToOwned::to_owned,
    );
    if pickup.count > 1 {
        format!("{name} x{}", pickup.count)
    } else {
        name
    }
}

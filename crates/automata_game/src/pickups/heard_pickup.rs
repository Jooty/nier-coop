use crate::pickups::ItemPickup;

/// What this game does with a pickup another player told it about: a line in the log, and for a
/// key item one of it for this player too. The item stays in this player's own world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeardPickup {
    pub sender_name: String,
    pub pickup: ItemPickup,
    pub grant_item_id: Option<u32>,
}

/// `None` for a pickup of this player's own, which its game has already shown.
pub fn heard_pickup(
    sender_slot: u8,
    local_slot: Option<u8>,
    sender_name: &str,
    pickup: ItemPickup,
) -> Option<HeardPickup> {
    if local_slot == Some(sender_slot) || !pickup.is_valid() {
        return None;
    }
    Some(HeardPickup {
        sender_name: sender_name.to_owned(),
        pickup,
        grant_item_id: pickup.key_item.then_some(pickup.item_id),
    })
}

/// `granted` is `None` for an ordinary item, else whether this player's game took the key item.
pub fn heard_pickup_line(
    sender_name: &str,
    item_name: Option<&str>,
    pickup: ItemPickup,
    granted: Option<bool>,
) -> String {
    let item = item_text(item_name, pickup);
    match granted {
        None => format!("{sender_name} picked up {item}"),
        Some(true) => format!("{sender_name} picked up {item} (key item), you have it too"),
        Some(false) => format!("{sender_name} picked up {item} (key item)"),
    }
}

/// The game shows its own line for a pickup; only a key item gets one more, once.
pub fn local_pickup_line(item_name: Option<&str>, pickup: ItemPickup) -> Option<String> {
    pickup.key_item.then(|| {
        format!(
            "{} is a key item, the others get it too",
            item_text(item_name, pickup)
        )
    })
}

fn item_text(item_name: Option<&str>, pickup: ItemPickup) -> String {
    let name = item_name
        .filter(|name| !name.trim().is_empty())
        .map_or_else(|| format!("item {}", pickup.item_id), str::to_owned);
    if pickup.count > 1 {
        format!("{name} x{}", pickup.count)
    } else {
        name
    }
}

use std::panic;

use rc_game::pickups::plan_partner_pickup;
use rc_proto::Message;

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::net;
use crate::pickups::hook::add_item_for_partner;

pub fn tick(_build: &VerifiedBuild) {
    let _ = panic::catch_unwind(crate::pickups::shared_items::tick);
    let Some(net) = net::handle() else {
        return;
    };
    let arrived = net.take_relayed(|message| matches!(message, Message::Pickup(_)));
    if arrived.is_empty() {
        return;
    }
    let name = net.view().peer_name.unwrap_or_default();
    for message in arrived {
        let Message::Pickup(pickup) = message else {
            continue;
        };
        let plan = plan_partner_pickup(&name, pickup);
        post_event(&plan.notice);
        log_line(&format!(
            "pickups: partner took item id {} x{}, grant {}",
            pickup.item_id,
            pickup.count,
            plan.grant.is_some()
        ));
        if let Some(grant) = plan.grant {
            let result = panic::catch_unwind(|| add_item_for_partner(grant.item_id, grant.count))
                .ok()
                .flatten();
            log_line(&format!(
                "pickups: key item id {} granted, add returned {result:?}",
                grant.item_id
            ));
        }
    }
}

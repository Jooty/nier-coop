use rc_proto::{ContainerFlag, ContainerItem, ContainerOpened};

use crate::containers::OpenedSet;
use crate::pickups::item_name;
use crate::stay_together::FollowRole;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkipReason {
    OtherZone,
    AlreadyOpenedHere,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceivePlan {
    Skip(SkipReason),
    Apply {
        action_id: u16,
        items: Vec<ContainerItem>,
        gold: u32,
        host_sets: Vec<ContainerFlag>,
        notice: Option<String>,
    },
}

pub fn plan_receive(
    role: FollowRole,
    opened: &ContainerOpened,
    local_phase: Option<u16>,
    set: &mut OpenedSet,
    partner_name: &str,
) -> ReceivePlan {
    if local_phase != Some(opened.phase) {
        return ReceivePlan::Skip(SkipReason::OtherZone);
    }
    if !set.claim(opened.action_id) {
        return ReceivePlan::Skip(SkipReason::AlreadyOpenedHere);
    }
    let host_sets = if role == FollowRole::Host {
        opened.flags.clone()
    } else {
        Vec::new()
    };
    let has_contents = !opened.items.is_empty() || opened.gold > 0;
    ReceivePlan::Apply {
        action_id: opened.action_id,
        items: opened.items.clone(),
        gold: opened.gold,
        host_sets,
        notice: has_contents.then(|| notice_text(partner_name, opened)),
    }
}

pub fn remove_only_message(phase: u16, action_id: u16) -> ContainerOpened {
    ContainerOpened {
        phase,
        action_id,
        items: Vec::new(),
        gold: 0,
        flags: Vec::new(),
    }
}

pub fn notice_text(partner_name: &str, opened: &ContainerOpened) -> String {
    let mut parts: Vec<String> = opened
        .items
        .iter()
        .map(|item| {
            let name = item_name(item.item_id)
                .map_or_else(|| format!("item {}", item.item_id), str::to_owned);
            if item.count > 1 {
                format!("{name} x{}", item.count)
            } else {
                name
            }
        })
        .collect();
    if opened.gold > 0 {
        parts.push(format!("{} gold", opened.gold));
    }
    format!("{partner_name} opened a container: {}", parts.join(", "))
}

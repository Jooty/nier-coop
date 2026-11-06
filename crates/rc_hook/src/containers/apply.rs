use rc_game::containers::{ReceivePlan, plan_receive};
use rc_game::stay_together::FollowRole;
use rc_proto::{ContainerOpened, StoryChange};

use crate::build_gate::VerifiedBuild;
use crate::containers::calls::{remove_glint, remove_use_point};
use crate::containers::hooks::add_money_for_partner;
use crate::containers::state::{OPENED, REGISTRY};
use crate::diagnostics::log_line;
use crate::event_log::post_event;
use crate::pickups::add_item_for_partner;
use crate::story_progress::apply_change;

pub(super) fn apply_notice(
    build: &VerifiedBuild,
    role: FollowRole,
    opened: &ContainerOpened,
    local_phase: Option<u16>,
    partner_name: &str,
) {
    let plan = {
        let mut set = OPENED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        plan_receive(role, opened, local_phase, &mut set, partner_name)
    };
    let ReceivePlan::Apply {
        action_id,
        items,
        gold,
        host_sets,
        notice,
    } = plan
    else {
        log_line(&format!(
            "containers: notice for action {} in zone {} dropped ({plan:?}), this zone is {local_phase:?}",
            opened.action_id, opened.phase
        ));
        return;
    };
    remove_use_point(build, i32::from(action_id));
    let glint = REGISTRY
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .glint_handle_of(i32::from(action_id));
    if let Some(handle) = glint {
        remove_glint(build, handle);
    }
    for item in &items {
        let added = add_item_for_partner(item.item_id, item.count);
        log_line(&format!(
            "containers: gave item id {} x{}, add returned {added:?}",
            item.item_id, item.count
        ));
    }
    if gold > 0 {
        let paid = i32::try_from(gold).is_ok_and(|amount| add_money_for_partner(build, amount));
        log_line(&format!("containers: gave {gold} gold, done {paid}"));
    }
    for flag in &host_sets {
        apply_change(
            build,
            StoryChange::Flag {
                array: flag.array,
                id: flag.id,
                on: true,
            },
        );
        log_line(&format!(
            "containers: host set {:?} flag {} for the guest's open",
            flag.array, flag.id
        ));
    }
    log_line(&format!(
        "containers: action {action_id} removed (glint {glint:?}), {} items, {gold} gold, {} flags",
        items.len(),
        host_sets.len()
    ));
    if let Some(notice) = notice {
        post_event(&notice);
    }
}

use hudhook::imgui::Ui;
use rc_game::stay_together::{MAX_LEASH_UNITS, MIN_LEASH_UNITS};
use rc_game::zone_rules::{MAX_ZONE_RADIUS_UNITS, MIN_ZONE_RADIUS_UNITS};

use crate::net::NetShared;
use crate::stay_together::{follow_enabled, leash, set_follow_enabled, set_leash, stay_status};
use crate::zone_rules::{set_zone_radius, zone_radius};

pub(super) fn draw_stay_together(ui: &Ui, view: &NetShared) {
    ui.separator();
    ui.text("stay together");
    let mut follow = follow_enabled();
    if ui.checkbox("follow the host's map changes", &mut follow) {
        set_follow_enabled(follow);
    }
    draw_role_and_status(ui, view);
    let mut radius_units = zone_radius().units();
    if ui.slider(
        "everyone nearby radius (units)",
        MIN_ZONE_RADIUS_UNITS,
        MAX_ZONE_RADIUS_UNITS,
        &mut radius_units,
    ) {
        set_zone_radius(radius_units);
    }
    let mut leash_units = leash().units();
    if ui.slider(
        "leash (units)",
        MIN_LEASH_UNITS,
        MAX_LEASH_UNITS,
        &mut leash_units,
    ) {
        set_leash(leash_units);
    }
}

fn draw_role_and_status(ui: &Ui, view: &NetShared) {
    ui.text(match view.local_slot {
        Some(0) => "you are the host: you change zones only when everyone is nearby",
        Some(_) => "you are the guest: only the host can change zones",
        None => "role: unknown until connected",
    });
    let status = stay_status();
    if !status.follow.is_empty() {
        ui.text_wrapped(&status.follow);
    }
    if status.blocked_exits > 0 {
        ui.text(format!("zone changes blocked: {}", status.blocked_exits));
    }
}

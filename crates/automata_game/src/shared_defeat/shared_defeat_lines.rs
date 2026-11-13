use crate::defeat::{DefeatFlow, RestorePoint};
use crate::load_gate::PlaceRelation;
use crate::shared_defeat::{
    DefeatWay, GameOverChoice, GameOverMenuWrite, HEARD_DEFEAT_NOTE_KEPT_FOR_MS,
    PartnerBodyGuardAct, PartnerBodyReading, RestorePlace,
};
use crate::travel::JumpRole;

pub fn shared_defeat_on_line() -> String {
    "shared defeat: on, the first game-over menu choice of either player is made on the other copy too, each copy logs its way on and its restore point at a defeat, and the partner body never starts this copy's defeat flow".to_owned()
}

pub fn shared_defeat_off_line() -> String {
    "shared defeat: shared_defeat is off, each copy answers its own game-over menu".to_owned()
}

pub fn restore_text(restore_point: RestorePoint, place: Option<RestorePlace>) -> String {
    match (restore_point, place) {
        (RestorePoint::Present, Some(place)) => format!("restore point {}", place.describe()),
        (RestorePoint::Present, None) => "a restore point whose place cannot be read".to_owned(),
        (RestorePoint::Absent, _) => "no restore point".to_owned(),
        (RestorePoint::Unreadable, _) => "the restore point cannot be read".to_owned(),
    }
}

pub fn told_restore_text(place: Option<RestorePlace>) -> String {
    match place {
        Some(place) => format!("restore point {}", place.describe()),
        None => "no restore point it could read".to_owned(),
    }
}

pub fn local_way_line(way: DefeatWay, restore: &str, partner: Option<&str>) -> String {
    let partner = partner.map_or_else(
        || "nothing heard yet from the other copy".to_owned(),
        |partner| format!("the other copy told: {partner}"),
    );
    format!(
        "shared defeat: this copy's flow takes {} ({restore}); {partner}",
        way.label()
    )
}

pub fn partner_way_text(way: DefeatWay, place: Option<RestorePlace>) -> String {
    format!("{} ({})", way.label(), told_restore_text(place))
}

pub fn partner_way_line(
    role: JumpRole,
    partner: &str,
    local_way: Option<DefeatWay>,
    local_restore: &str,
    relation: PlaceRelation,
) -> String {
    let local = local_way.map_or_else(
        || format!("this copy runs no way of its flow yet ({local_restore})"),
        |way| format!("this copy takes {} ({local_restore})", way.label()),
    );
    let places = match (relation, role) {
        (PlaceRelation::Same, _) => "the two restore points name the same place",
        (PlaceRelation::Other, JumpRole::Guest) => {
            "the two restore points name other places: after the reload the catch-up brings this guest to the host's place"
        }
        (PlaceRelation::Other, _) => {
            "the two restore points name other places: after the reload the catch-up brings the guest to this host's place"
        }
        (PlaceRelation::Unknown, _) => "the two restore points cannot be compared",
    };
    format!("shared defeat: the other copy's flow takes {partner}; {local}; {places}")
}

pub fn ways_differ_line(local: DefeatWay, partner: DefeatWay) -> String {
    format!(
        "shared defeat: the two copies take other ways on ({} here, {} there); each goes its own way, nothing is changed",
        local.label(),
        partner.label()
    )
}

pub fn own_choice_line(choice: GameOverChoice, local_unattended: bool) -> String {
    let attended = if local_unattended {
        " (this copy counts as unattended)"
    } else {
        ""
    };
    format!(
        "shared defeat: this copy's player chose {} in the game-over menu{attended}; told to the other copy",
        choice.label()
    )
}

pub fn heard_choice_line(choice: GameOverChoice, flow: Option<DefeatFlow>) -> String {
    format!(
        "shared defeat: heard the other copy's game-over choice {}; it is made here once this copy's menu waits for a choice (for at most {} s; flow at {})",
        choice.label(),
        HEARD_DEFEAT_NOTE_KEPT_FOR_MS / 1000,
        flow_text(flow)
    )
}

pub fn heard_choice_after_own_line(heard: GameOverChoice, own: GameOverChoice) -> String {
    format!(
        "shared defeat: heard the other copy's game-over choice {}, but this copy already took {}; nothing is written",
        heard.label(),
        own.label()
    )
}

pub fn own_choice_first_line(heard: GameOverChoice, own: GameOverChoice) -> String {
    format!(
        "shared defeat: this copy's player chose {} before the heard choice {} was made here; the heard one is dropped",
        own.label(),
        heard.label()
    )
}

pub fn follow_line(
    choice: GameOverChoice,
    write: GameOverMenuWrite,
    waited_ms: u64,
    local_unattended: bool,
) -> String {
    let attended = if local_unattended {
        "this copy counts as unattended"
    } else {
        "this copy's player had not chosen yet"
    };
    format!(
        "shared defeat: makes the other copy's choice {} in this copy's game-over menu ({}; heard {waited_ms} ms ago; {attended})",
        choice.label(),
        write.label()
    )
}

pub fn retry_not_followed_line() -> String {
    "shared defeat: the other copy chose retry, but the in-place retry bit is clear here, so this copy's retry would open its second list; nothing is written, this copy keeps its own menu".to_owned()
}

pub fn heard_choice_waits_line(
    choice: GameOverChoice,
    waited_ms: u64,
    flow: Option<DefeatFlow>,
) -> String {
    format!(
        "shared defeat: waits {waited_ms} ms with the other copy's choice {} until this copy's menu waits for a choice (gives up after {} s; flow at {})",
        choice.label(),
        HEARD_DEFEAT_NOTE_KEPT_FOR_MS / 1000,
        flow_text(flow)
    )
}

pub fn heard_choice_gave_up_line(choice: GameOverChoice, flow: Option<DefeatFlow>) -> String {
    format!(
        "shared defeat: gave up on the other copy's choice {} after {} s, this copy's menu never waited for a choice (flow at {}); this copy keeps its own menu",
        choice.label(),
        HEARD_DEFEAT_NOTE_KEPT_FOR_MS / 1000,
        flow_text(flow)
    )
}

pub fn heard_choice_dropped_for_the_way_line(choice: GameOverChoice, way: DefeatWay) -> String {
    format!(
        "shared defeat: this copy's flow takes {}, it has no menu; the other copy's choice {} is dropped",
        way.label(),
        choice.label()
    )
}

pub fn local_flow_over_line() -> String {
    "shared defeat: this copy's defeat flow is over".to_owned()
}

pub fn no_partner_any_more_line() -> String {
    "shared defeat: no campaign partner any more; this copy keeps its own flow and menu".to_owned()
}

pub fn menu_write_done_line(write: GameOverMenuWrite, written: bool) -> String {
    if written {
        format!(
            "shared defeat: wrote the menu's {} (each write the game's own)",
            write.label()
        )
    } else {
        format!(
            "shared defeat: the menu's {} could not be written (a write failed); this copy keeps its own menu",
            write.label()
        )
    }
}

pub fn partner_body_guard_line(
    act: PartnerBodyGuardAct,
    body: PartnerBodyReading,
    written: bool,
) -> Option<String> {
    let what = match act {
        PartnerBodyGuardAct::Nothing => return None,
        PartnerBodyGuardAct::DisarmTheFlightDefeatTimer => format!(
            "the partner body is in flight state {:#X} with its defeat timer at {}; the timer is set to 0 so it never asks for this copy's defeat flow",
            body.state, body.flight_defeat_timer
        ),
        PartnerBodyGuardAct::KeepTheHealthAtOne => format!(
            "the partner body is in state {:#X} with health {}; its health is set to 1 so a hit never asks for this copy's defeat flow",
            body.state, body.health
        ),
    };
    Some(format!("shared defeat: guard: {what} (written {written})"))
}

fn flow_text(flow: Option<DefeatFlow>) -> String {
    flow.map_or_else(
        || "a flow that cannot be read".to_owned(),
        DefeatFlow::describe,
    )
}

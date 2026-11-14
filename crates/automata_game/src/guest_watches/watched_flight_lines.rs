use crate::guest_watches::WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS;
use crate::partner_flight::{FlightCall, FlightReport};

pub fn watched_hero_flight_setting_line(on: bool) -> String {
    if on {
        format!(
            "watching: watched_hero_follows_flight is on: while watching, the hero gets on or off its flight unit and takes the flight mode of a shooting stage as the host's flight report says, once they have differed for {WATCHED_HERO_FLIGHT_FOLLOWS_AFTER_MS} ms (bounded tries), and in free flight its unit follows the host's unit"
        )
    } else {
        "watching: watched_hero_follows_flight is off: the hero's flight is left to its game and the fed input".to_owned()
    }
}

pub fn flight_report_label(report: &FlightReport) -> String {
    if report.is_in_a_unit() {
        format!("{} in mode {}", report.kind.label(), report.mode.label())
    } else {
        report.kind.label().to_owned()
    }
}

pub fn flight_call_label(call: FlightCall) -> String {
    match call {
        FlightCall::Board => "startPhaseEventFlightUnit".to_owned(),
        FlightCall::GetOff => "endPhaseEventFlightUnit".to_owned(),
        FlightCall::SetMode(mode) => format!("the {} mode setter", mode.label()),
    }
}

pub fn hero_flight_try_line(
    call: FlightCall,
    host: &FlightReport,
    hero: &FlightReport,
    hero_state: i32,
    differed_ms: u64,
    (try_number, most): (u32, u32),
) -> String {
    format!(
        "watching: the host's hero is {} and this game's hero is {} (state {hero_state:#X}) for {differed_ms} ms; called {} on the hero (try {try_number} of {most})",
        flight_report_label(host),
        flight_report_label(hero),
        flight_call_label(call)
    )
}

pub fn hero_flight_after_call_line(
    call: FlightCall,
    hero: &FlightReport,
    hero_state: i32,
) -> String {
    format!(
        "watching: after {} the hero is {} (state {hero_state:#X})",
        flight_call_label(call),
        flight_report_label(hero)
    )
}

pub fn hero_flight_refused_line(call: FlightCall, most: u32) -> String {
    format!(
        "watching: the hero did not follow the host's flight after {most} tries of {}; it is not called again until the host's flight report changes",
        flight_call_label(call)
    )
}

pub fn hero_stays_in_its_shooting_stage_line(hero_state: i32, differed_ms: u64) -> String {
    format!(
        "watching: the host's hero is out of its flight unit and this game's hero is still in a shooting stage (state {hero_state:#X}) for {differed_ms} ms; nothing is called: endPhaseEventFlightUnit would leave the stage flag set and the game would start a stage state without a unit"
    )
}

pub fn hero_flight_mode_left_alone_line(host: &FlightReport, hero: &FlightReport) -> String {
    format!(
        "watching: the host's hero is {} and this game's hero is {}; the mode is left alone outside a shooting stage on both (a mode setter starts a shooting stage state)",
        flight_report_label(host),
        flight_report_label(hero)
    )
}

pub fn host_flight_report_is_empty_line(host_state: u16) -> String {
    format!(
        "watching: the host's hero is in flight state {host_state:#X} but its flight report is empty (its partner_flight_unit is off?); the hero's flight is left to its game"
    )
}

pub fn hero_unit_follows_line(whole_way: bool) -> String {
    format!(
        "watching: first write of the hero's flight unit toward the host's unit ({}; free flight only, logged once)",
        if whole_way {
            "the whole way"
        } else {
            "a nudge"
        }
    )
}

pub fn watching_flight_part(hero: &FlightReport, host: Option<&FlightReport>) -> String {
    format!(
        "; flight {} (hero), {} (host)",
        flight_report_label(hero),
        host.map_or_else(|| "not told".to_owned(), flight_report_label)
    )
}

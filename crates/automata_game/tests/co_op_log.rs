use automata_game::address::{HUD_LOG_POST_TEXT_ID_FUNCTION, UI_TEXT_LOOKUP_FUNCTION};
use automata_game::co_op_log::{
    CoOpLines, GAME_LOG_TEXT_SLOT_COUNT, OverlayLog, PlayerSeen, SessionSeen, game_log_text_id,
    game_log_text_slot, game_log_text_units, overlay_log_layout,
};
use automata_game::exe::ExeVariant;

const HERE: i32 = 0x100;
const ELSEWHERE: i32 = 0x220;

fn player(phase_id: i32, health: i32) -> Option<PlayerSeen> {
    Some(PlayerSeen { phase_id, health })
}

fn seen(local: Option<PlayerSeen>, partner: Option<PlayerSeen>) -> SessionSeen<'static> {
    SessionSeen {
        connected: true,
        partner_name: "Jay",
        local,
        partner,
    }
}

fn not_linked() -> SessionSeen<'static> {
    SessionSeen {
        connected: false,
        partner_name: "Jay",
        local: player(HERE, 100),
        partner: None,
    }
}

fn joined() -> CoOpLines {
    let mut lines = CoOpLines::new();
    lines.note(&seen(player(HERE, 100), player(HERE, 100)), 0);
    lines.note(&seen(player(HERE, 100), player(HERE, 100)), 5_000);
    lines
}

#[test]
fn the_partner_joins_once_their_first_state_is_heard() {
    let mut lines = CoOpLines::new();
    assert!(lines.note(&seen(player(HERE, 100), None), 0).is_empty());
    assert_eq!(
        lines.note(&seen(player(HERE, 100), player(HERE, 100)), 10),
        vec!["Jay joined"]
    );
    assert!(
        lines
            .note(&seen(player(HERE, 100), player(HERE, 100)), 20)
            .is_empty()
    );
}

#[test]
fn the_partner_leaves_when_the_link_drops_and_can_join_again() {
    let mut lines = joined();
    assert_eq!(lines.note(&not_linked(), 6_000), vec!["Jay left"]);
    assert!(lines.note(&not_linked(), 6_100).is_empty());
    assert_eq!(
        lines.note(&seen(player(HERE, 100), player(HERE, 100)), 7_000),
        vec!["Jay joined"]
    );
}

#[test]
fn nobody_leaves_a_session_that_never_had_a_partner() {
    let mut lines = CoOpLines::new();
    assert!(lines.note(&not_linked(), 0).is_empty());
}

#[test]
fn another_place_is_told_once_it_has_held_and_so_is_the_return() {
    let mut lines = joined();
    let apart = seen(player(HERE, 100), player(ELSEWHERE, 100));
    assert!(lines.note(&apart, 6_000).is_empty());
    assert!(lines.note(&apart, 7_000).is_empty());
    assert_eq!(lines.note(&apart, 8_000), vec!["Jay went to another place"]);
    assert!(lines.note(&apart, 9_000).is_empty());
    let together = seen(player(ELSEWHERE, 100), player(ELSEWHERE, 100));
    assert!(lines.note(&together, 10_000).is_empty());
    assert_eq!(lines.note(&together, 12_000), vec!["Jay is here again"]);
}

#[test]
fn a_place_that_differs_only_for_a_moment_is_not_told() {
    let mut lines = joined();
    let apart = seen(player(HERE, 100), player(ELSEWHERE, 100));
    let together = seen(player(ELSEWHERE, 100), player(ELSEWHERE, 100));
    assert!(lines.note(&apart, 6_000).is_empty());
    assert!(lines.note(&together, 7_000).is_empty());
    assert!(lines.note(&together, 20_000).is_empty());
}

#[test]
fn a_partner_who_joins_from_another_place_is_told_as_being_there() {
    let mut lines = CoOpLines::new();
    let apart = seen(player(HERE, 100), player(ELSEWHERE, 100));
    assert_eq!(lines.note(&apart, 0), vec!["Jay joined"]);
    assert_eq!(lines.note(&apart, 2_000), vec!["Jay is in another place"]);
}

#[test]
fn a_defeat_and_the_return_from_it_are_told_once_each() {
    let mut lines = joined();
    let down = seen(player(HERE, 100), player(HERE, 0));
    assert_eq!(lines.note(&down, 6_000), vec!["Jay was defeated"]);
    assert!(lines.note(&down, 6_100).is_empty());
    assert_eq!(
        lines.note(&seen(player(HERE, 100), player(HERE, 80)), 9_000),
        vec!["Jay is back up"]
    );
}

#[test]
fn a_partner_first_seen_without_health_was_not_defeated_here() {
    let mut lines = CoOpLines::new();
    assert_eq!(
        lines.note(&seen(player(HERE, 100), player(HERE, 0)), 0),
        vec!["Jay joined"]
    );
}

#[test]
fn the_players_own_defeat_is_told_with_or_without_a_partner() {
    let mut lines = CoOpLines::new();
    let mut alone = not_linked();
    assert!(lines.note(&alone, 0).is_empty());
    alone.local = player(HERE, 0);
    assert_eq!(lines.note(&alone, 100), vec!["You were defeated"]);
    assert!(lines.note(&alone, 200).is_empty());
}

#[test]
fn the_mods_text_ids_are_its_own_and_wrap_around_its_slots() {
    assert_eq!(game_log_text_slot(game_log_text_id(0)), Some(0));
    assert_eq!(game_log_text_slot(game_log_text_id(15)), Some(15));
    assert_eq!(
        game_log_text_id(GAME_LOG_TEXT_SLOT_COUNT + 3),
        game_log_text_id(3)
    );
    assert_eq!(game_log_text_slot(game_log_text_id(15) + 1), None);
    assert_eq!(game_log_text_slot(game_log_text_id(0) - 1), None);
    assert_eq!(game_log_text_slot(0), None);
    assert_ne!(game_log_text_id(0), 0);
}

#[test]
fn a_line_for_the_games_log_is_bounded_utf16_with_a_zero_at_the_end() {
    assert_eq!(game_log_text_units("Jay"), vec![74, 97, 121, 0]);
    assert_eq!(
        game_log_text_units("<J>a\n%y\\"),
        game_log_text_units("Jay")
    );
    let long = game_log_text_units(&"x".repeat(500));
    assert_eq!(long.len(), 121);
    assert_eq!(long.last(), Some(&0));
    let cut_inside_a_pair = format!("{}\u{1F600}", "x".repeat(119));
    let units = game_log_text_units(&cut_inside_a_pair);
    assert_eq!(units.len(), 120);
    assert_eq!(units[118], u16::from(b'x'));
}

#[test]
fn the_overlay_log_keeps_the_newest_lines_and_drops_a_repeat() {
    let mut log = OverlayLog::new();
    assert!(log.post("Jay joined", 0));
    assert!(!log.post("Jay joined", 1_000));
    assert!(log.post("Jay joined", 2_000));
    assert!(!log.post("\n", 2_000));
    for index in 0..6 {
        assert!(log.post(&format!("line {index}"), 3_000));
    }
    let shown = log.visible(3_100);
    assert_eq!(shown.len(), 6);
    assert_eq!(shown[0].text, "line 0");
}

#[test]
fn an_overlay_line_fades_in_stays_and_fades_out() {
    let mut log = OverlayLog::new();
    log.post("Jay joined", 0);
    assert!(log.visible(50)[0].alpha < 1.0);
    assert_eq!(log.visible(1_000)[0].alpha, 1.0);
    assert!(log.visible(4_500)[0].alpha < 1.0);
    assert!(log.visible(5_000).is_empty());
}

#[test]
fn overlay_lines_stack_upwards_and_scale_with_the_screen() {
    let small = overlay_log_layout(1080.0);
    let large = overlay_log_layout(2160.0);
    assert!(small.line_top(1) < small.line_top(0));
    assert_eq!(large.left, small.left * 2.0);
    assert_eq!(large.line_height, small.line_height * 2.0);
}

#[test]
fn the_log_addresses_are_the_ones_matched_by_bytes_in_both_exes() {
    assert_eq!(
        HUD_LOG_POST_TEXT_ID_FUNCTION.rva(ExeVariant::Main),
        0x8CA480
    );
    assert_eq!(
        HUD_LOG_POST_TEXT_ID_FUNCTION.rva(ExeVariant::Compat),
        0x8C1C80
    );
    assert_eq!(UI_TEXT_LOOKUP_FUNCTION.rva(ExeVariant::Main), 0x9BD770);
    assert_eq!(UI_TEXT_LOOKUP_FUNCTION.rva(ExeVariant::Compat), 0x9B4EC0);
}

#[test]
fn a_down_and_a_revive_have_their_lines_and_the_rest_have_none() {
    use automata_game::co_op_log::downed_effect_line;
    use automata_game::downed::{DownedEffect, FallBackCause, GotUpCause};
    let line = |effect| downed_effect_line(effect, "Jay");
    assert_eq!(
        line(DownedEffect::WentDown).as_deref(),
        Some("You were downed")
    );
    assert_eq!(
        line(DownedEffect::GotUp(GotUpCause::Revived)).as_deref(),
        Some("Jay revived you")
    );
    assert_eq!(
        line(DownedEffect::PartnerWentDown).as_deref(),
        Some("Jay was downed")
    );
    assert_eq!(
        line(DownedEffect::ReviveDone).as_deref(),
        Some("You revived Jay")
    );
    assert_eq!(
        line(DownedEffect::FellBack(FallBackCause::BothDown)).as_deref(),
        Some("Nobody could revive you")
    );
    assert_eq!(line(DownedEffect::GotUp(GotUpCause::PartnerGone)), None);
    assert_eq!(line(DownedEffect::PartnerGotUp), None);
    assert_eq!(line(DownedEffect::ReviveBroken), None);
    assert_eq!(line(DownedEffect::ReviveProgress(50)), None);
}

#[test]
fn a_downed_hero_held_at_one_health_is_not_called_defeated() {
    let mut lines = CoOpLines::new();
    lines.note(&seen(player(HERE, 100), player(HERE, 100)), 0);
    assert!(
        lines
            .note(&seen(player(HERE, 100), player(HERE, 1)), 100)
            .is_empty()
    );
    assert!(
        lines
            .note(&seen(player(HERE, 1), player(HERE, 1)), 200)
            .is_empty()
    );
}

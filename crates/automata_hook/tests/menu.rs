use automata_game::exe::ExeVariant;
use automata_game::save_redirect::SaveRedirectStatus;
use automata_hook::menu::AutomataMenu;
use nc_hook::menu::{
    CoOpStatus, FixOutcome, GameMenu, PlayerLine, SECTION_ORDER, SectionBody, SectionKind,
    build_menu,
};

fn menu() -> AutomataMenu {
    AutomataMenu::new(Some(ExeVariant::Main), "abc123")
}

fn empty_note_of(kind: SectionKind) -> Option<String> {
    build_menu(&menu())
        .sections
        .into_iter()
        .find(|section| section.kind == kind)
        .and_then(|section| section.empty_note)
}

#[test]
fn every_shared_section_is_present_in_order() {
    let kinds: Vec<SectionKind> = build_menu(&menu())
        .sections
        .iter()
        .map(|section| section.kind)
        .collect();
    assert_eq!(kinds, SECTION_ORDER.to_vec());
}

#[test]
fn a_section_with_nothing_says_so() {
    assert!(empty_note_of(SectionKind::Teleport).is_some_and(|note| note.contains("no areas")));
    assert!(
        empty_note_of(SectionKind::StoryJump).is_some_and(|note| note.contains("no story jumps"))
    );
    assert!(empty_note_of(SectionKind::Fixes).is_some_and(|note| note.contains("no fix")));
    assert!(empty_note_of(SectionKind::CoOp).is_some_and(|note| note.contains("no other player")));
}

#[test]
fn the_settings_section_has_content_and_no_empty_note() {
    assert_eq!(empty_note_of(SectionKind::SettingsAndLogs), None);
}

#[test]
fn status_lines_name_the_variant_the_build_the_link_the_saves_and_the_start() {
    let lines = menu().status_lines();
    assert_eq!(lines.len(), 5);
    assert_eq!(lines[0], "exe variant: main");
    assert_eq!(lines[1], "build: abc123");
    assert!(lines[2].starts_with("link: "));
    assert_eq!(
        lines[3],
        "saves: NOT redirected, the normal save folder is in use"
    );
    assert_eq!(lines[4], "start: not started");
}

#[test]
fn the_co_op_section_and_the_link_line_show_what_the_link_says() {
    let linked = menu().with_co_op_status(|| CoOpStatus {
        link: "connected".to_owned(),
        players: vec![PlayerLine {
            name: "Mira".to_owned(),
            detail: "9S, health 4200".to_owned(),
        }],
        ping_ms: Some(42),
    });
    assert_eq!(linked.status_lines()[2], "link: connected");
    let status = linked.co_op_status();
    assert_eq!(status.players[0].detail, "9S, health 4200");
    assert_eq!(status.ping_ms, Some(42));
    assert_eq!(
        menu().status_lines()[2],
        "link: not started (the launcher gave no link port)"
    );
}

#[test]
fn the_start_line_shows_what_the_start_state_says() {
    let started = menu().with_start_status(|| "start: waiting for the title".to_owned());
    assert_eq!(started.status_lines()[4], "start: waiting for the title");
}

#[test]
fn the_saves_line_shows_the_co_op_folder_once_the_redirect_is_installed() {
    let installed = menu().with_save_status(|| SaveRedirectStatus::CoopFolder);
    assert_eq!(installed.status_lines()[3], "saves: co-op folder");
}

#[test]
fn the_compat_and_unknown_variants_are_named() {
    let compat = AutomataMenu::new(Some(ExeVariant::Compat), "x").status_lines();
    assert_eq!(compat[0], "exe variant: compat");
    let unknown = AutomataMenu::new(None, "x").status_lines();
    assert_eq!(unknown[0], "exe variant: unknown");
}

#[test]
fn the_empty_sections_are_the_bodies_of_the_model() {
    for section in build_menu(&menu()).sections {
        match section.body {
            SectionBody::Teleport { current, areas } => {
                assert!(current.is_none());
                assert!(areas.is_empty());
            }
            SectionBody::StoryJump(jumps) => assert!(jumps.is_empty()),
            SectionBody::Fixes(fixes) => assert!(fixes.is_empty()),
            SectionBody::CoOp(status) => assert!(status.players.is_empty()),
            SectionBody::SettingsAndLogs { panels, .. } => assert!(panels.is_empty()),
        }
    }
}

#[test]
fn actions_that_need_content_are_refused_in_plain_words() {
    let menu = menu();
    assert_eq!(menu.go_to_partner(), Err("no partner yet".to_owned()));
    assert_eq!(menu.bring_partner(), Err("no partner yet".to_owned()));
    assert_eq!(menu.run_story_jump(0), Err("no story jumps yet".to_owned()));
    assert_eq!(menu.run_fix(0), FixOutcome::Refused("no fixes".to_owned()));
}

use automata_game::exe::ExeVariant;
use automata_game::save_redirect::SaveRedirectStatus;
use automata_game::start::PlayReadings;
use automata_game::travel::{
    FAST_TRAVEL_IDLE_STATE, JumpRole, LoadedPoints, TravelPoint, TravelView, WorldReadings,
};
use automata_hook::menu::{AutomataMenu, DownedHooks, GIVE_UP_FIX_ID, TeleportHooks};
use nc_hook::menu::{
    AreaChangeRefusal, AreaEntry, CoOpStatus, FixOutcome, GameMenu, MenuNote, PlayerLine,
    SECTION_ORDER, SectionBody, SectionKind, build_menu,
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
    assert_eq!(lines.len(), 8);
    assert_eq!(lines[0], "exe variant: main");
    assert_eq!(lines[1], "build: abc123");
    assert!(lines[2].starts_with("link: "));
    assert_eq!(
        lines[3],
        "saves: NOT redirected, the normal save folder is in use"
    );
    assert_eq!(lines[4], "start: not started");
    assert_eq!(lines[5], "story: gate not installed");
    assert_eq!(lines[6], "teleport: nothing asked yet");
    assert_eq!(lines[7], "partner body: no session");
}

#[test]
fn the_partner_body_line_shows_what_the_partner_driver_says() {
    let stand_in = menu()
        .with_partner_body_status(|| "partner body: the engine's 9S stands in for A2".to_owned());
    assert_eq!(
        stand_in.status_lines()[7],
        "partner body: the engine's 9S stands in for A2"
    );
}

#[test]
fn the_story_line_shows_what_the_gate_says() {
    let held = menu().with_story_status(|| "story: held".to_owned());
    assert_eq!(held.status_lines()[5], "story: held");
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

fn loaded_world() -> WorldReadings {
    WorldReadings {
        play: PlayReadings {
            scene_state: 1,
            phase_id: 0x210,
            scene_loaded: 1,
            player_present: true,
        },
        fast_travel_state: Some(FAST_TRAVEL_IDLE_STATE),
        fast_travel_step: Some(6),
        loaded: LoadedPoints::of_names(["ft_AP", "ft_RC", "re_made_up"]),
        paused: false,
    }
}

fn host_view() -> TravelView {
    TravelView {
        role: JumpRole::Host,
        world: Some(loaded_world()),
        partner_body_present: true,
        ..TravelView::default()
    }
}

fn hooks(view: fn() -> TravelView) -> TeleportHooks {
    TeleportHooks {
        view,
        ask_jump: |point| {
            if point.code() == "ft_AP" {
                Ok(())
            } else {
                Err(format!("asked for {}", point.code()))
            }
        },
        ask_go_to_partner: || Ok("asked to go".to_owned()),
        ask_bring_partner: || Ok("asked to bring".to_owned()),
    }
}

fn area(code: &str) -> AreaEntry {
    let point = TravelPoint::from_code(code).expect("a known code");
    AreaEntry {
        id: u32::from(point.index()),
        name: point.label(),
    }
}

#[test]
fn the_area_list_is_the_games_loaded_travel_points_under_their_codes() {
    let menu = menu().with_teleport(hooks(host_view));
    assert_eq!(menu.areas(), vec![area("ft_RC"), area("ft_AP")]);
    assert_eq!(menu.areas()[1].name, "Travel point ft_AP");
    assert_eq!(menu.current_area(), None);
    assert_eq!(menu.area_change_blocker(), None);
    assert!(!menu.area_change_in_progress());
    assert_eq!(menu.change_area(&area("ft_AP")), Ok(()));
    assert_eq!(
        menu.change_area(&area("ft_RC")),
        Err("asked for ft_RC".to_owned())
    );
}

#[test]
fn the_list_is_empty_with_its_own_line_until_the_world_is_loaded() {
    let menu = menu().with_teleport(hooks(TravelView::default));
    assert!(menu.areas().is_empty());
    let note = build_menu(&menu)
        .sections
        .into_iter()
        .find(|section| section.kind == SectionKind::Teleport)
        .and_then(|section| section.empty_note)
        .expect("the empty list says why");
    assert!(note.contains("no travel points yet"), "{note}");
    assert_eq!(
        menu.area_change_blocker(),
        Some(AreaChangeRefusal::Other("no world yet".to_owned()))
    );
}

#[test]
fn a_guest_and_a_point_that_is_not_loaded_are_refused_with_the_reason() {
    let guest = menu().with_teleport(hooks(|| TravelView {
        role: JumpRole::Guest,
        ..host_view()
    }));
    assert_eq!(
        guest.area_change_blocker(),
        Some(AreaChangeRefusal::NotHost)
    );
    assert_eq!(
        guest.change_area(&area("ft_AP")),
        Err("only the host may jump".to_owned())
    );
    let host = menu().with_teleport(hooks(host_view));
    assert_eq!(
        host.change_area(&area("ft_BK")),
        Err("ft_BK is not among the points the game holds now".to_owned())
    );
    assert_eq!(
        host.change_area(&AreaEntry {
            id: 99,
            name: "made up".to_owned()
        }),
        Err("unknown travel point 99".to_owned())
    );
}

#[test]
fn a_jump_under_way_and_its_arrival_reach_the_shared_tracker() {
    let under_way = menu().with_teleport(hooks(|| TravelView {
        jump_under_way: true,
        ..host_view()
    }));
    assert!(under_way.area_change_in_progress());
    assert_eq!(
        under_way.area_change_blocker(),
        Some(AreaChangeRefusal::Other(
            "refused while a jump is under way".to_owned()
        ))
    );
    let arrived = menu().with_teleport(hooks(|| TravelView {
        current: TravelPoint::from_code("ft_AP"),
        ..host_view()
    }));
    assert_eq!(arrived.current_area(), Some(area("ft_AP")));
}

#[test]
fn going_to_the_partner_is_refused_in_plain_words_without_a_partner_body() {
    let alone = menu().with_teleport(hooks(|| TravelView {
        partner_body_present: false,
        ..host_view()
    }));
    assert_eq!(
        alone.go_to_partner(),
        Err("not moved: there is no partner body yet".to_owned())
    );
    let together = menu().with_teleport(hooks(host_view));
    assert_eq!(together.go_to_partner(), Ok("asked to go".to_owned()));
    assert_eq!(together.bring_partner(), Ok("asked to bring".to_owned()));
}

#[test]
fn what_happened_last_is_a_note_of_the_section_and_a_status_line() {
    let menu = menu().with_teleport(hooks(|| TravelView {
        move_line: "the other player brought you to them".to_owned(),
        ..host_view()
    }));
    assert_eq!(
        menu.section_notes(SectionKind::Teleport),
        vec![MenuNote::plain("the other player brought you to them")]
    );
    assert!(menu.section_notes(SectionKind::CoOp).is_empty());
    assert_eq!(
        menu.status_lines()[6],
        "teleport: the other player brought you to them"
    );
}

#[test]
fn giving_up_is_offered_only_to_a_downed_hero_and_asks_once() {
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    static HERO_IS_DOWN: AtomicBool = AtomicBool::new(false);
    static TIMES_ASKED: AtomicU32 = AtomicU32::new(0);
    let menu = menu().with_downed(DownedHooks {
        hero_is_down: || HERO_IS_DOWN.load(Ordering::Relaxed),
        ask_to_give_up: || {
            TIMES_ASKED.fetch_add(1, Ordering::Relaxed);
        },
    });
    assert!(menu.fixes().is_empty());
    assert_eq!(
        menu.run_fix(GIVE_UP_FIX_ID),
        FixOutcome::Refused("the hero is not down".to_owned())
    );
    HERO_IS_DOWN.store(true, Ordering::Relaxed);
    let fixes = menu.fixes();
    assert_eq!(fixes.len(), 1);
    assert_eq!(fixes[0].id, GIVE_UP_FIX_ID);
    assert!(fixes[0].label.starts_with("Give up"));
    assert_eq!(
        menu.run_fix(GIVE_UP_FIX_ID + 1),
        FixOutcome::Refused("no fixes".to_owned())
    );
    assert_eq!(TIMES_ASKED.load(Ordering::Relaxed), 0);
    assert_eq!(menu.run_fix(GIVE_UP_FIX_ID), FixOutcome::Done);
    assert_eq!(TIMES_ASKED.load(Ordering::Relaxed), 1);
}

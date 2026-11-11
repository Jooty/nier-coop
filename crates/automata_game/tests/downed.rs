use automata_game::address::{BLOWN_DOWN_STATE_HANDLER, STATE_DISPATCH_TABLE};
use automata_game::downed::{
    BLEED_OUT_SECONDS, BLOWN_DOWN_STATE_HANDLER_START, DOWNED_POSE_START_INTERVAL_MS,
    DOWNED_POSE_STATE, DownedEffect, DownedPhase, DownedPrompt, DownedRules, DownedSignal,
    DownedView, FallBackCause, GotUpCause, HeroGuard, HeroHit, HeroHitTreatment,
    PoseHandlerReading, REVIVE_HOLD_SECONDS, REVIVE_RANGE_UNITS, StepInput, distance_between,
    downed_pose_start, downed_prompt, downed_revive_from_text, effect_log_line,
    handler_called_by_the_state_case, hero_hit_treatment, hero_is_defeated_by_the_game,
    hit_put_the_hero_down, pose_handler_is_the_noted_one, pose_is_started_again, prompt_text,
    revived_health, start_is_refused_in_the_downed_pose,
};
use automata_game::exe::ExeVariant;

const STEP_SECONDS: f32 = 0.25;
const FULL_HEALTH: i32 = 400;

const PARTNER_UP: DownedSignal = DownedSignal {
    revives: true,
    phase: DownedPhase::Up,
    times_downed: 0,
    reviving_down: 0,
    reviving_percent: 0,
};

fn input(partner: Option<DownedSignal>) -> StepInput {
    StepInput {
        dt_seconds: STEP_SECONDS,
        enabled: true,
        partner,
        partner_distance: Some(1.0),
        local_health: FULL_HEALTH,
        lethal_hit_seen: false,
        defeated_by_the_game: false,
    }
}

fn lethal_hit(partner: Option<DownedSignal>) -> StepInput {
    StepInput {
        local_health: 1,
        lethal_hit_seen: true,
        ..input(partner)
    }
}

fn down(partner: Option<DownedSignal>) -> StepInput {
    StepInput {
        local_health: 1,
        ..input(partner)
    }
}

fn downed_rules() -> DownedRules {
    let mut rules = DownedRules::default();
    rules.step(input(Some(PARTNER_UP)));
    assert_eq!(
        rules.step(lethal_hit(Some(PARTNER_UP))),
        vec![DownedEffect::WentDown]
    );
    rules
}

fn partner_downed(times_downed: u8) -> DownedSignal {
    DownedSignal {
        phase: DownedPhase::Downed,
        times_downed,
        ..PARTNER_UP
    }
}

fn steps_until(rules: &mut DownedRules, step: StepInput, wanted: DownedEffect) -> u32 {
    for count in 1..=100 {
        if rules.step(step).contains(&wanted) {
            return count;
        }
    }
    panic!("{wanted:?} never came");
}

#[test]
fn a_lethal_hit_puts_the_hero_down_while_the_partner_is_up_and_revives() {
    let rules = downed_rules();
    assert_eq!(rules.local(), DownedPhase::Downed);
    let signal = rules.signal(true);
    assert_eq!(signal.phase, DownedPhase::Downed);
    assert_eq!(signal.times_downed, 1);
    assert!(signal.revives);
}

#[test]
fn nobody_goes_down_without_a_partner_who_can_revive() {
    let cannot_revive = [
        None,
        Some(DownedSignal {
            revives: false,
            ..PARTNER_UP
        }),
        Some(partner_downed(1)),
        Some(DownedSignal {
            phase: DownedPhase::Defeated,
            ..PARTNER_UP
        }),
    ];
    for partner in cannot_revive {
        let mut rules = DownedRules::default();
        assert!(
            !rules
                .step(lethal_hit(partner))
                .contains(&DownedEffect::WentDown)
        );
        assert_eq!(rules.local(), DownedPhase::Up);
    }
}

#[test]
fn the_setting_switched_off_keeps_the_hero_up_and_tells_the_partner() {
    let mut rules = DownedRules::default();
    let step = StepInput {
        enabled: false,
        ..lethal_hit(Some(PARTNER_UP))
    };
    assert!(rules.step(step).is_empty());
    assert_eq!(rules.hero_guard(), HeroGuard::Off);
    assert!(!rules.signal(false).revives);
}

#[test]
fn low_health_without_a_lethal_hit_is_not_a_down() {
    let mut rules = DownedRules::default();
    assert!(rules.step(down(Some(PARTNER_UP))).is_empty());
    let healthy = StepInput {
        lethal_hit_seen: true,
        ..input(Some(PARTNER_UP))
    };
    assert!(rules.step(healthy).is_empty());
}

#[test]
fn the_defeat_is_held_off_only_while_the_partner_is_up() {
    let mut rules = DownedRules::default();
    assert_eq!(rules.hero_guard(), HeroGuard::Off);
    rules.step(input(Some(PARTNER_UP)));
    assert_eq!(rules.hero_guard(), HeroGuard::HoldOffDefeat);
    rules.step(input(Some(partner_downed(1))));
    assert_eq!(rules.hero_guard(), HeroGuard::Off);
    rules.step(input(None));
    assert_eq!(rules.hero_guard(), HeroGuard::Off);
    assert_eq!(downed_rules().hero_guard(), HeroGuard::Shield);
}

#[test]
fn standing_close_for_the_hold_time_revives_the_partner() {
    let mut rules = DownedRules::default();
    rules.step(input(Some(PARTNER_UP)));
    let first = rules.step(input(Some(partner_downed(1))));
    assert_eq!(
        first,
        vec![DownedEffect::PartnerWentDown, DownedEffect::ReviveStarted]
    );
    let steps = steps_until(
        &mut rules,
        input(Some(partner_downed(1))),
        DownedEffect::ReviveDone,
    );
    assert_eq!(steps + 1, (REVIVE_HOLD_SECONDS / STEP_SECONDS) as u32);
    let signal = rules.signal(true);
    assert_eq!((signal.reviving_down, signal.reviving_percent), (1, 100));
}

#[test]
fn the_revive_percent_stays_below_100_until_the_revive_is_done() {
    let mut rules = DownedRules::default();
    let mut highest_before_done = 0;
    for _ in 0..100 {
        let effects = rules.step(input(Some(partner_downed(1))));
        if effects.contains(&DownedEffect::ReviveDone) {
            break;
        }
        highest_before_done = rules.signal(true).reviving_percent;
    }
    assert!((75..100).contains(&highest_before_done));
}

#[test]
fn a_done_revive_is_held_until_the_partner_is_up_and_then_forgotten() {
    let mut rules = DownedRules::default();
    steps_until(
        &mut rules,
        input(Some(partner_downed(1))),
        DownedEffect::ReviveDone,
    );
    let far = StepInput {
        partner_distance: Some(50.0),
        ..input(Some(partner_downed(1)))
    };
    assert!(rules.step(far).is_empty());
    assert_eq!(rules.signal(true).reviving_percent, 100);
    assert_eq!(
        rules.step(input(Some(PARTNER_UP))),
        vec![DownedEffect::PartnerGotUp]
    );
    assert_eq!(rules.signal(true).reviving_percent, 0);
}

#[test]
fn walking_away_breaks_the_revive_and_it_starts_from_nothing() {
    let mut rules = DownedRules::default();
    let close = input(Some(partner_downed(1)));
    for _ in 0..6 {
        rules.step(close);
    }
    assert!(rules.reviving_progress() > 0.4);
    for distance in [Some(REVIVE_RANGE_UNITS + 0.1), None] {
        rules.step(close);
        let away = StepInput {
            partner_distance: distance,
            ..close
        };
        assert_eq!(rules.step(away), vec![DownedEffect::ReviveBroken]);
        assert_eq!(rules.reviving_progress(), 0.0);
        assert!(rules.step(away).is_empty());
    }
    assert_eq!(rules.step(close), vec![DownedEffect::ReviveStarted]);
}

#[test]
fn the_edge_of_the_range_still_revives() {
    let mut rules = DownedRules::default();
    let at_the_edge = StepInput {
        partner_distance: Some(REVIVE_RANGE_UNITS),
        ..input(Some(partner_downed(1)))
    };
    assert!(
        rules
            .step(at_the_edge)
            .contains(&DownedEffect::ReviveStarted)
    );
}

#[test]
fn a_new_down_of_the_partner_starts_a_new_revive() {
    let mut rules = DownedRules::default();
    steps_until(
        &mut rules,
        input(Some(partner_downed(1))),
        DownedEffect::ReviveDone,
    );
    let effects = rules.step(input(Some(partner_downed(2))));
    assert_eq!(effects, vec![DownedEffect::ReviveStarted]);
    let signal = rules.signal(true);
    assert_eq!(signal.reviving_down, 2);
    assert!(signal.reviving_percent < 100);
}

#[test]
fn a_done_revive_for_this_down_gets_the_hero_up() {
    let mut rules = downed_rules();
    let reviving = DownedSignal {
        reviving_down: 1,
        reviving_percent: 50,
        ..PARTNER_UP
    };
    assert_eq!(
        rules.step(down(Some(reviving))),
        vec![DownedEffect::BeingRevived(50)]
    );
    assert_eq!(rules.being_revived_progress(), 0.5);
    assert!(rules.step(down(Some(reviving))).is_empty());
    let done = DownedSignal {
        reviving_percent: 100,
        ..reviving
    };
    assert_eq!(
        rules.step(down(Some(done))),
        vec![DownedEffect::GotUp(GotUpCause::Revived)]
    );
    assert_eq!(rules.local(), DownedPhase::Up);
    assert_eq!(rules.being_revived_progress(), 0.0);
}

#[test]
fn a_done_revive_of_an_earlier_down_does_not_get_the_hero_up() {
    let mut rules = downed_rules();
    let stale = DownedSignal {
        reviving_down: 0,
        reviving_percent: 100,
        ..PARTNER_UP
    };
    assert!(rules.step(down(Some(stale))).is_empty());
    assert_eq!(rules.local(), DownedPhase::Downed);
    assert_eq!(rules.being_revived_progress(), 0.0);
}

#[test]
fn the_second_down_has_its_own_number() {
    let mut rules = downed_rules();
    let done = DownedSignal {
        reviving_down: 1,
        reviving_percent: 100,
        ..PARTNER_UP
    };
    rules.step(down(Some(done)));
    assert_eq!(
        rules.step(lethal_hit(Some(done))),
        vec![DownedEffect::WentDown]
    );
    assert_eq!(rules.signal(true).times_downed, 2);
    assert!(rules.step(down(Some(done))).is_empty());
    assert_eq!(rules.local(), DownedPhase::Downed);
}

#[test]
fn both_down_falls_back_to_the_games_defeat() {
    let mut rules = downed_rules();
    assert_eq!(
        rules.step(down(Some(partner_downed(1)))),
        vec![
            DownedEffect::PartnerWentDown,
            DownedEffect::FellBack(FallBackCause::BothDown)
        ]
    );
    assert_eq!(rules.local(), DownedPhase::Defeated);
    assert_eq!(rules.signal(true).phase, DownedPhase::Defeated);
    assert_eq!(rules.hero_guard(), HeroGuard::Off);
}

#[test]
fn a_partner_defeated_by_its_game_makes_the_downed_hero_fall_back() {
    let mut rules = downed_rules();
    let defeated = DownedSignal {
        phase: DownedPhase::Defeated,
        ..PARTNER_UP
    };
    assert_eq!(
        rules.step(down(Some(defeated))),
        vec![DownedEffect::FellBack(FallBackCause::PartnerDefeated)]
    );
}

#[test]
fn giving_up_falls_back_only_for_a_downed_hero() {
    let mut rules = DownedRules::default();
    rules.ask_to_give_up();
    assert!(rules.step(input(Some(PARTNER_UP))).is_empty());
    let mut rules = downed_rules();
    rules.ask_to_give_up();
    assert_eq!(
        rules.step(down(Some(PARTNER_UP))),
        vec![DownedEffect::FellBack(FallBackCause::GaveUp)]
    );
}

#[test]
fn a_partner_who_is_gone_or_stops_reviving_gets_the_downed_hero_up() {
    let gone = [
        None,
        Some(DownedSignal {
            revives: false,
            ..PARTNER_UP
        }),
    ];
    for partner in gone {
        let mut rules = downed_rules();
        assert_eq!(
            rules.step(down(partner)),
            vec![DownedEffect::GotUp(GotUpCause::PartnerGone)]
        );
        assert_eq!(rules.local(), DownedPhase::Up);
    }
}

#[test]
fn a_defeat_by_the_game_while_down_falls_back() {
    let mut rules = downed_rules();
    let defeated = StepInput {
        defeated_by_the_game: true,
        local_health: 0,
        ..input(Some(PARTNER_UP))
    };
    assert_eq!(
        rules.step(defeated),
        vec![DownedEffect::FellBack(FallBackCause::DefeatedByTheGame)]
    );
}

#[test]
fn a_defeat_by_the_game_while_up_is_told_and_ends_when_the_hero_is_back() {
    let mut rules = DownedRules::default();
    rules.step(input(Some(PARTNER_UP)));
    let defeated = StepInput {
        defeated_by_the_game: true,
        local_health: 0,
        ..input(Some(PARTNER_UP))
    };
    assert_eq!(rules.step(defeated), vec![DownedEffect::DefeatedWhileUp]);
    assert_eq!(rules.signal(true).phase, DownedPhase::Defeated);
    assert!(rules.step(defeated).is_empty());
    let still_low = StepInput {
        local_health: 1,
        ..input(Some(PARTNER_UP))
    };
    assert!(rules.step(still_low).is_empty());
    assert_eq!(
        rules.step(input(Some(PARTNER_UP))),
        vec![DownedEffect::BackFromTheGamesDefeat]
    );
    assert_eq!(rules.local(), DownedPhase::Up);
}

#[test]
fn a_defeated_hero_revives_nobody() {
    let mut rules = DownedRules::default();
    let defeated = StepInput {
        defeated_by_the_game: true,
        local_health: 0,
        ..input(Some(partner_downed(1)))
    };
    let effects = rules.step(defeated);
    assert!(!effects.contains(&DownedEffect::ReviveStarted));
    assert_eq!(rules.reviving_progress(), 0.0);
}

#[test]
fn the_wait_fills_in_the_bleed_out_time_and_is_only_shown() {
    let mut rules = downed_rules();
    let steps = (BLEED_OUT_SECONDS / STEP_SECONDS) as u32;
    for _ in 0..steps / 2 {
        rules.step(down(Some(PARTNER_UP)));
    }
    assert!((rules.bleed_out_progress() - 0.5).abs() < 0.01);
    for _ in 0..steps {
        assert!(rules.step(down(Some(PARTNER_UP))).is_empty());
    }
    assert_eq!(rules.bleed_out_progress(), 1.0);
    assert_eq!(rules.local(), DownedPhase::Downed);
}

#[test]
fn a_long_frame_counts_as_a_quarter_second_at_most() {
    let mut rules = DownedRules::default();
    let long = StepInput {
        dt_seconds: 30.0,
        ..input(Some(partner_downed(1)))
    };
    assert!(!rules.step(long).contains(&DownedEffect::ReviveDone));
}

#[test]
fn an_invalid_signal_counts_as_no_partner() {
    let invalid = DownedSignal {
        reviving_percent: 101,
        ..PARTNER_UP
    };
    assert!(!invalid.is_valid());
    let mut rules = DownedRules::default();
    assert!(rules.step(lethal_hit(Some(invalid))).is_empty());
}

#[test]
fn a_revived_hero_gets_a_quarter_of_its_health_and_at_least_two() {
    assert_eq!(revived_health(400), 100);
    assert_eq!(revived_health(4), 2);
    assert_eq!(revived_health(0), 2);
}

#[test]
fn the_distance_is_the_straight_line_and_none_for_a_broken_number() {
    assert_eq!(
        distance_between([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]),
        Some(5.0)
    );
    assert_eq!(distance_between([f32::NAN, 0.0, 0.0], [0.0; 3]), None);
}

#[test]
fn hits_on_the_hero_are_floored_kept_off_or_left_alone_by_the_guard() {
    let on_foot = HeroHit::default();
    let flying = HeroHit {
        flight_flags: [0, 1],
        ..on_foot
    };
    assert_eq!(
        hero_hit_treatment(HeroGuard::Off, on_foot),
        HeroHitTreatment::Untouched
    );
    assert_eq!(
        hero_hit_treatment(HeroGuard::HoldOffDefeat, on_foot),
        HeroHitTreatment::Floored
    );
    assert_eq!(
        hero_hit_treatment(HeroGuard::HoldOffDefeat, flying),
        HeroHitTreatment::Untouched
    );
    assert_eq!(
        hero_hit_treatment(HeroGuard::Shield, flying),
        HeroHitTreatment::KeptOff
    );
    for guard in [HeroGuard::Off, HeroGuard::HoldOffDefeat, HeroGuard::Shield] {
        assert_eq!(HeroGuard::from_number(guard.as_number()), guard);
    }
}

#[test]
fn a_floored_hit_puts_the_hero_down_only_where_the_game_would_have_defeated_it() {
    let plain = HeroHit::default();
    assert!(hit_put_the_hero_down(plain, 30, 1));
    assert!(hit_put_the_hero_down(plain, 1, 1));
    assert!(!hit_put_the_hero_down(plain, 30, 2));
    assert!(!hit_put_the_hero_down(plain, 0, 1));
    let floored_by_the_game = [
        HeroHit {
            caller_asks_for_no_defeat: true,
            ..plain
        },
        HeroHit {
            berserk_switch: 1,
            ..plain
        },
        HeroHit {
            own_floor_switch: 1,
            ..plain
        },
        HeroHit {
            own_floor_mode: 1,
            ..plain
        },
    ];
    for hit in floored_by_the_game {
        assert!(!hit_put_the_hero_down(hit, 30, 1));
    }
    let other_mode = HeroHit {
        own_floor_mode: 2,
        ..plain
    };
    assert!(hit_put_the_hero_down(other_mode, 30, 1));
}

#[test]
fn a_body_kept_down_leaves_the_pose_state_by_no_start() {
    assert!(start_is_refused_in_the_downed_pose(true, DOWNED_POSE_STATE));
    assert!(!start_is_refused_in_the_downed_pose(
        false,
        DOWNED_POSE_STATE
    ));
    assert!(!start_is_refused_in_the_downed_pose(true, 0));
    let start = downed_pose_start();
    assert_eq!(i32::from(start.state), DOWNED_POSE_STATE);
    assert_eq!((start.variant, start.b, start.c), (0, 0, 0));
}

#[test]
fn the_pose_is_started_again_only_outside_it_and_once_a_second() {
    assert!(pose_is_started_again(0, 5_000, None));
    assert!(!pose_is_started_again(DOWNED_POSE_STATE, 5_000, None));
    assert!(!pose_is_started_again(
        0,
        5_000,
        Some(5_000 - DOWNED_POSE_START_INTERVAL_MS + 1)
    ));
    assert!(pose_is_started_again(
        0,
        5_000,
        Some(5_000 - DOWNED_POSE_START_INTERVAL_MS)
    ));
}

#[test]
fn the_pose_handler_is_read_from_the_dispatcher_case_of_both_exes() {
    let main_case = [0x48, 0x8B, 0xCF, 0xE8, 0xE8, 0x7B, 0xFA, 0xFF];
    let compat_case = [0x48, 0x8B, 0xCF, 0xE8, 0x98, 0x7D, 0xFA, 0xFF];
    assert_eq!(
        handler_called_by_the_state_case(0x50_4440, &main_case),
        Some(BLOWN_DOWN_STATE_HANDLER.rva(ExeVariant::Main))
    );
    assert_eq!(
        handler_called_by_the_state_case(0x4F_E2A0, &compat_case),
        Some(BLOWN_DOWN_STATE_HANDLER.rva(ExeVariant::Compat))
    );
    assert_eq!(
        handler_called_by_the_state_case(0x50_4440, &[0x48; 8]),
        None
    );
    assert_eq!(
        handler_called_by_the_state_case(0x50_4440, &main_case[..6]),
        None
    );
    assert_eq!(BLOWN_DOWN_STATE_HANDLER.rva(ExeVariant::Main), 0x4A_C030);
    assert_eq!(BLOWN_DOWN_STATE_HANDLER.rva(ExeVariant::Compat), 0x4A_6040);
    assert_eq!(STATE_DISPATCH_TABLE.rva(ExeVariant::Main), 0x50_7124);
    assert_eq!(STATE_DISPATCH_TABLE.rva(ExeVariant::Compat), 0x50_0F84);
}

#[test]
fn the_pose_is_trusted_only_with_the_noted_handler() {
    let noted = PoseHandlerReading {
        state_case_rva: Some(0x50_4440),
        state_case_bytes: vec![0x48, 0x8B, 0xCF, 0xE8, 0xE8, 0x7B, 0xFA, 0xFF],
        handler_bytes: BLOWN_DOWN_STATE_HANDLER_START.to_vec(),
    };
    assert!(pose_handler_is_the_noted_one(&noted, 0x4A_C030));
    assert!(!pose_handler_is_the_noted_one(&noted, 0x4A_C031));
    let other_code = PoseHandlerReading {
        handler_bytes: vec![0x90; 24],
        ..noted.clone()
    };
    assert!(!pose_handler_is_the_noted_one(&other_code, 0x4A_C030));
    let unread = PoseHandlerReading {
        state_case_rva: None,
        ..noted
    };
    assert!(!pose_handler_is_the_noted_one(&unread, 0x4A_C030));
}

#[test]
fn the_prompt_follows_the_rules() {
    let view = DownedView::default();
    assert_eq!(downed_prompt(view), DownedPrompt::Nothing);
    let down = DownedView {
        local: DownedPhase::Downed,
        partner: Some(DownedPhase::Up),
        ..view
    };
    assert_eq!(downed_prompt(down), DownedPrompt::Down);
    let long = DownedView {
        bleed_out_progress: 1.0,
        ..down
    };
    assert_eq!(downed_prompt(long), DownedPrompt::DownForLong);
    let revived = DownedView {
        being_revived_progress: 0.5,
        ..long
    };
    assert_eq!(downed_prompt(revived), DownedPrompt::BeingRevived(0.5));
    let partner_down = DownedView {
        partner: Some(DownedPhase::Downed),
        ..view
    };
    assert_eq!(downed_prompt(partner_down), DownedPrompt::PartnerDown);
    let reviving = DownedView {
        reviving_progress: 0.25,
        ..partner_down
    };
    assert_eq!(downed_prompt(reviving), DownedPrompt::RevivingPartner(0.25));
    let defeated = DownedView {
        local: DownedPhase::Defeated,
        ..reviving
    };
    assert_eq!(downed_prompt(defeated), DownedPrompt::Nothing);
}

#[test]
fn the_prompt_texts_name_the_partner_and_only_revives_have_a_bar() {
    assert_eq!(prompt_text(DownedPrompt::Nothing, "Kim"), None);
    assert_eq!(
        prompt_text(DownedPrompt::PartnerDown, "Kim").as_deref(),
        Some("Kim is DOWNED: stand close to revive")
    );
    assert_eq!(
        prompt_text(DownedPrompt::Down, "Kim").as_deref(),
        Some("DOWNED: Kim can revive you")
    );
    assert!(
        prompt_text(DownedPrompt::DownForLong, "Kim").is_some_and(|text| text.contains("give up"))
    );
    assert_eq!(DownedPrompt::Down.bar(), None);
    assert_eq!(DownedPrompt::RevivingPartner(0.4).bar(), Some(0.4));
    assert_eq!(DownedPrompt::BeingRevived(0.6).bar(), Some(0.6));
}

#[test]
fn every_step_has_a_log_line_that_says_downed() {
    let effects = [
        DownedEffect::WentDown,
        DownedEffect::GotUp(GotUpCause::Revived),
        DownedEffect::GotUp(GotUpCause::PartnerGone),
        DownedEffect::FellBack(FallBackCause::BothDown),
        DownedEffect::FellBack(FallBackCause::PartnerDefeated),
        DownedEffect::FellBack(FallBackCause::GaveUp),
        DownedEffect::FellBack(FallBackCause::DefeatedByTheGame),
        DownedEffect::DefeatedWhileUp,
        DownedEffect::BackFromTheGamesDefeat,
        DownedEffect::BeingRevived(50),
        DownedEffect::PartnerWentDown,
        DownedEffect::PartnerGotUp,
        DownedEffect::PartnerFellBack,
        DownedEffect::ReviveStarted,
        DownedEffect::ReviveProgress(25),
        DownedEffect::ReviveBroken,
        DownedEffect::ReviveDone,
    ];
    let lines: Vec<String> = effects.into_iter().map(effect_log_line).collect();
    assert!(lines.iter().all(|line| line.starts_with("downed: ")));
    for (index, line) in lines.iter().enumerate() {
        assert!(!lines[..index].contains(line), "two steps share {line:?}");
    }
    assert!(effect_log_line(DownedEffect::FellBack(FallBackCause::BothDown)).contains("both"));
}

#[test]
fn the_setting_reads_on_and_off_only() {
    assert_eq!(downed_revive_from_text("on"), Some(true));
    assert_eq!(downed_revive_from_text("off"), Some(false));
    assert_eq!(downed_revive_from_text("yes"), None);
}

#[test]
fn the_games_defeat_is_its_state_or_flow_and_for_a_downed_hero_also_lost_health() {
    assert!(!hero_is_defeated_by_the_game(
        DownedPhase::Up,
        0,
        false,
        false
    ));
    assert!(hero_is_defeated_by_the_game(
        DownedPhase::Up,
        50,
        true,
        false
    ));
    assert!(hero_is_defeated_by_the_game(
        DownedPhase::Up,
        50,
        false,
        true
    ));
    assert!(hero_is_defeated_by_the_game(
        DownedPhase::Downed,
        0,
        false,
        false
    ));
    assert!(!hero_is_defeated_by_the_game(
        DownedPhase::Downed,
        1,
        false,
        false
    ));
}

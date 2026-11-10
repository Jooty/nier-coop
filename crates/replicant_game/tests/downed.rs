use replicant_game::Replicant;
use replicant_game::downed::{
    BLEED_OUT_SECONDS, DOWNED_HEALTH, DownedEffect, DownedRules, DownedState, HEARTBEAT_SECONDS,
    INCOMING_REVIVE_PROGRESS_STALE_SECONDS, PLAYER_CONTROL_DISABLE_NATIVE_RVAS,
    PLAYER_CONTROL_ENABLE_NATIVE_RVAS, PlayerLock, PlayerLockCall, REVIVE_HOLD_SECONDS,
    REVIVE_RANGE_UNITS, StepInput, revived_health,
};
use replicant_game::offsets::{
    LIFE_FLAG_ENDURE_BIT, LIFE_FLAG_INVULNERABLE_BIT, PLAYER_PARAM_ARRAY_RVA,
    PLAYER_PARAM_LIFE_FLAGS_OFFSET,
};
use replicant_proto::CombatEvent;

use crate::fake_memory::FakeMemory;

const FULL_HEALTH: i32 = 400;
const TICK_SECONDS: f32 = 0.4;

fn step(health: i32, distance: Option<f32>) -> StepInput {
    StepInput {
        dt_seconds: TICK_SECONDS,
        session_active: true,
        local_health: health,
        lethal_hit_seen: health == DOWNED_HEALTH,
        partner_distance: distance,
    }
}

fn without_heartbeats(effects: Vec<DownedEffect>) -> Vec<DownedEffect> {
    effects
        .into_iter()
        .filter(|effect| {
            !matches!(
                effect,
                DownedEffect::Send(CombatEvent::Up | CombatEvent::ReviveProgress(_))
            )
        })
        .collect()
}

fn downed_rules() -> DownedRules {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH, None));
    let effects = rules.step(step(DOWNED_HEALTH, None));
    assert_eq!(
        effects,
        vec![
            DownedEffect::EnterDowned,
            DownedEffect::Send(CombatEvent::Downed)
        ]
    );
    rules
}

fn rules_with_downed_partner() -> DownedRules {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH, None));
    rules.on_partner_event(CombatEvent::Downed);
    rules
}

#[test]
fn a_lethal_hit_floored_at_one_health_downs_the_player() {
    let rules = downed_rules();
    assert_eq!(rules.local(), DownedState::Downed);
}

#[test]
fn starting_a_session_at_one_health_is_not_a_hit() {
    let mut rules = DownedRules::new();
    assert!(rules.step(step(DOWNED_HEALTH, None)).is_empty());
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn real_death_at_zero_health_is_not_downed() {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH, None));
    assert!(rules.step(step(0, None)).is_empty());
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn outside_a_session_nothing_happens() {
    let mut rules = DownedRules::new();
    let mut input = step(FULL_HEALTH, None);
    input.session_active = false;
    assert!(rules.step(input).is_empty());
    input.local_health = DOWNED_HEALTH;
    input.lethal_hit_seen = true;
    assert!(rules.step(input).is_empty());
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn a_revive_from_the_partner_gets_the_downed_player_up() {
    let mut rules = downed_rules();
    let effects = rules.on_partner_event(CombatEvent::Revived);
    assert_eq!(effects, vec![DownedEffect::LeaveDowned]);
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn staying_close_for_the_hold_time_revives_the_partner() {
    let mut rules = rules_with_downed_partner();
    let close = Some(REVIVE_RANGE_UNITS);
    let ticks = (REVIVE_HOLD_SECONDS / TICK_SECONDS).ceil() as usize;
    for _ in 0..ticks - 1 {
        assert!(without_heartbeats(rules.step(step(FULL_HEALTH, close))).is_empty());
    }
    assert!(rules.revive_progress() > 0.8);
    let effects = without_heartbeats(rules.step(step(FULL_HEALTH, close)));
    assert_eq!(effects, vec![DownedEffect::Send(CombatEvent::Revived)]);
    assert_eq!(rules.partner(), DownedState::Up);
}

#[test]
fn stepping_away_or_changing_phase_resets_the_revive_hold() {
    let mut rules = rules_with_downed_partner();
    rules.step(step(FULL_HEALTH, Some(1.0)));
    assert!(rules.revive_progress() > 0.0);
    rules.step(step(FULL_HEALTH, Some(REVIVE_RANGE_UNITS + 1.0)));
    assert_eq!(rules.revive_progress(), 0.0);
    rules.step(step(FULL_HEALTH, Some(1.0)));
    rules.step(step(FULL_HEALTH, None));
    assert_eq!(rules.revive_progress(), 0.0);
}

#[test]
fn the_panel_button_revives_the_partner_at_once() {
    let mut rules = rules_with_downed_partner();
    let effects = rules.request_revive_partner();
    assert_eq!(effects, vec![DownedEffect::Send(CombatEvent::Revived)]);
    assert!(rules.request_revive_partner().is_empty());
}

#[test]
fn both_players_downed_runs_the_game_over_flow_once() {
    let mut rules = downed_rules();
    let effects = rules.on_partner_event(CombatEvent::Downed);
    assert_eq!(
        effects,
        vec![
            DownedEffect::Send(CombatEvent::GameOver),
            DownedEffect::ForceKill
        ]
    );
    assert!(rules.is_game_over());
    assert!(rules.step(step(DOWNED_HEALTH, None)).is_empty());
    assert!(rules.on_partner_event(CombatEvent::Downed).is_empty());
}

#[test]
fn going_down_after_the_partner_is_already_down_ends_the_fight() {
    let mut rules = rules_with_downed_partner();
    let effects = rules.step(step(DOWNED_HEALTH, None));
    assert_eq!(effects.last(), Some(&DownedEffect::ForceKill));
    assert!(rules.is_game_over());
}

#[test]
fn a_game_over_from_the_partner_kills_us_too() {
    let mut rules = DownedRules::new();
    assert_eq!(
        rules.on_partner_event(CombatEvent::GameOver),
        vec![DownedEffect::ForceKill]
    );
    assert!(rules.on_partner_event(CombatEvent::GameOver).is_empty());
}

#[test]
fn healthy_again_after_game_over_arms_the_rules_afresh() {
    let mut rules = downed_rules();
    rules.on_partner_event(CombatEvent::Downed);
    assert!(rules.step(step(0, None)).is_empty());
    rules.step(step(FULL_HEALTH, None));
    assert!(!rules.is_game_over());
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn leaving_the_session_while_downed_gets_the_player_up_again() {
    let mut rules = downed_rules();
    let mut input = step(DOWNED_HEALTH, None);
    input.session_active = false;
    assert_eq!(rules.step(input), vec![DownedEffect::LeaveDowned]);
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn the_test_button_downs_the_player_once() {
    let mut rules = DownedRules::new();
    assert_eq!(
        rules.request_go_down(),
        vec![
            DownedEffect::EnterDowned,
            DownedEffect::Send(CombatEvent::Downed)
        ]
    );
    assert!(rules.request_go_down().is_empty());
}

#[test]
fn revived_health_is_a_quarter_of_the_maximum_and_at_least_two() {
    assert_eq!(revived_health(400), 100);
    assert_eq!(revived_health(4), 2);
    assert_eq!(revived_health(1), 2);
}

#[test]
fn life_flags_set_and_clear_one_bit_without_touching_the_rest() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(
        PLAYER_PARAM_ARRAY_RVA + PLAYER_PARAM_LIFE_FLAGS_OFFSET,
        &[0x07],
    );
    let mut game = Replicant::new(memory);
    game.set_life_flag(LIFE_FLAG_ENDURE_BIT, true)
        .expect("writes");
    assert!(game.life_flag(LIFE_FLAG_ENDURE_BIT).expect("reads"));
    assert!(!game.life_flag(LIFE_FLAG_INVULNERABLE_BIT).expect("reads"));
    game.set_life_flag(LIFE_FLAG_INVULNERABLE_BIT, true)
        .expect("writes");
    game.set_life_flag(LIFE_FLAG_ENDURE_BIT, false)
        .expect("writes");
    let byte = game
        .memory()
        .get_at_rva(PLAYER_PARAM_ARRAY_RVA + PLAYER_PARAM_LIFE_FLAGS_OFFSET, 1)[0];
    assert_eq!(byte, 0x07 | LIFE_FLAG_INVULNERABLE_BIT);
}

#[test]
fn dropping_to_one_health_without_a_lethal_hit_is_not_downed() {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH, None));
    let mut input = step(DOWNED_HEALTH, None);
    input.lethal_hit_seen = false;
    assert!(rules.step(input).is_empty());
    assert_eq!(rules.local(), DownedState::Up);
}

#[test]
fn the_current_state_is_resent_every_heartbeat() {
    let mut rules = DownedRules::new();
    let mut sent = Vec::new();
    let ticks = (HEARTBEAT_SECONDS / TICK_SECONDS).ceil() as usize;
    for _ in 0..ticks {
        sent.extend(rules.step(step(FULL_HEALTH, None)));
    }
    assert_eq!(sent, vec![DownedEffect::Send(CombatEvent::Up)]);
    let mut rules = downed_rules();
    let mut sent = Vec::new();
    for _ in 0..ticks {
        sent.extend(rules.step(step(DOWNED_HEALTH, None)));
    }
    assert_eq!(sent, vec![DownedEffect::Send(CombatEvent::Downed)]);
}

#[test]
fn a_lost_downed_message_is_healed_by_the_heartbeat() {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH, None));
    rules.on_partner_event(CombatEvent::Downed);
    assert_eq!(rules.partner(), DownedState::Downed);
    rules.on_partner_event(CombatEvent::Up);
    assert_eq!(rules.partner(), DownedState::Up);
}

#[test]
fn a_repeated_downed_heartbeat_does_not_reset_the_revive_hold() {
    let mut rules = rules_with_downed_partner();
    rules.step(step(FULL_HEALTH, Some(1.0)));
    let progress = rules.revive_progress();
    rules.on_partner_event(CombatEvent::Downed);
    assert_eq!(rules.revive_progress(), progress);
}

#[test]
fn a_lost_revived_message_is_retried_when_the_downed_heartbeat_arrives() {
    let mut rules = rules_with_downed_partner();
    rules.request_revive_partner();
    assert_eq!(rules.partner(), DownedState::Up);
    rules.on_partner_event(CombatEvent::Downed);
    assert_eq!(rules.partner(), DownedState::Downed);
    assert_eq!(
        rules.request_revive_partner(),
        vec![DownedEffect::Send(CombatEvent::Revived)]
    );
}

#[test]
fn bleeding_out_never_kills_while_the_partner_is_up() {
    let mut rules = downed_rules();
    let ticks = (BLEED_OUT_SECONDS / TICK_SECONDS).ceil() as usize * 3;
    for _ in 0..ticks {
        let effects = rules.step(step(DOWNED_HEALTH, None));
        assert!(!effects.contains(&DownedEffect::ForceKill));
    }
    assert!(!rules.is_game_over());
    assert_eq!(rules.local(), DownedState::Downed);
    assert_eq!(rules.bleed_out_progress(), 1.0);
}

#[test]
fn a_revive_long_after_the_bleed_cap_still_gets_the_player_up() {
    let mut rules = downed_rules();
    let ticks = (90.0 / TICK_SECONDS).ceil() as usize;
    for _ in 0..ticks {
        rules.step(step(DOWNED_HEALTH, None));
    }
    let effects = rules.on_partner_event(CombatEvent::Revived);
    assert_eq!(effects, vec![DownedEffect::LeaveDowned]);
    assert_eq!(rules.local(), DownedState::Up);
    assert_eq!(rules.bleed_out_progress(), 0.0);
}

#[test]
fn after_the_bleed_cap_the_partner_going_down_ends_the_fight_once() {
    let mut rules = downed_rules();
    let ticks = (BLEED_OUT_SECONDS / TICK_SECONDS).ceil() as usize + 5;
    for _ in 0..ticks {
        rules.step(step(DOWNED_HEALTH, None));
    }
    let effects = rules.on_partner_event(CombatEvent::Downed);
    assert_eq!(
        effects,
        vec![
            DownedEffect::Send(CombatEvent::GameOver),
            DownedEffect::ForceKill
        ]
    );
    assert!(rules.on_partner_event(CombatEvent::Downed).is_empty());
}

#[test]
fn a_guest_never_kills_itself_when_both_are_down_it_waits_for_the_host() {
    let mut rules = DownedRules::new();
    rules.set_guest(true);
    rules.step(step(FULL_HEALTH, None));
    rules.step(step(DOWNED_HEALTH, None));
    let effects = rules.on_partner_event(CombatEvent::Downed);
    assert!(!effects.contains(&DownedEffect::ForceKill));
    assert!(rules.is_game_over());
}

#[test]
fn a_revive_restarts_the_bleed_out_clock() {
    let mut rules = downed_rules();
    for _ in 0..50 {
        rules.step(step(DOWNED_HEALTH, None));
    }
    assert!(rules.bleed_out_progress() > 0.0);
    rules.on_partner_event(CombatEvent::Revived);
    assert_eq!(rules.bleed_out_progress(), 0.0);
}

#[test]
fn a_downed_player_is_locked_and_revive_releases_it() {
    let mut rules = downed_rules();
    assert!(rules.locks_controls());
    rules.on_partner_event(CombatEvent::Revived);
    assert!(!rules.locks_controls());
}

#[test]
fn an_up_player_is_never_locked() {
    let mut rules = DownedRules::new();
    rules.step(step(FULL_HEALTH, None));
    assert!(!rules.locks_controls());
}

#[test]
fn the_end_of_the_session_releases_the_lock() {
    let mut rules = downed_rules();
    rules.step(StepInput {
        session_active: false,
        ..step(DOWNED_HEALTH, None)
    });
    assert!(!rules.locks_controls());
}

#[test]
fn game_over_releases_the_lock() {
    let mut rules = downed_rules();
    rules.on_partner_event(CombatEvent::GameOver);
    assert!(!rules.locks_controls());
}

#[test]
fn every_disable_native_has_an_enable_native_before_it() {
    assert_eq!(
        PLAYER_CONTROL_DISABLE_NATIVE_RVAS.len(),
        PLAYER_CONTROL_ENABLE_NATIVE_RVAS.len()
    );
    for (disable, enable) in PLAYER_CONTROL_DISABLE_NATIVE_RVAS
        .iter()
        .zip(PLAYER_CONTROL_ENABLE_NATIVE_RVAS)
    {
        assert_eq!(*disable, enable + 0x10);
    }
}

#[test]
fn entering_downed_disables_once_and_leaving_enables_once() {
    let mut lock = PlayerLock::new();
    assert_eq!(lock.step(false, true), None);
    assert_eq!(lock.step(true, true), Some(PlayerLockCall::Disable));
    assert_eq!(lock.step(true, true), None);
    assert!(lock.is_disabled());
    assert_eq!(lock.step(false, true), Some(PlayerLockCall::Enable));
    assert_eq!(lock.step(false, true), None);
    assert!(!lock.is_disabled());
}

#[test]
fn a_lock_that_could_not_be_called_is_retried_until_it_can() {
    let mut lock = PlayerLock::new();
    assert_eq!(lock.step(true, false), None);
    assert!(!lock.is_disabled());
    assert_eq!(lock.step(true, true), Some(PlayerLockCall::Disable));
    assert_eq!(lock.step(false, false), None);
    assert!(lock.is_disabled());
    assert_eq!(lock.step(false, true), Some(PlayerLockCall::Enable));
}

#[test]
fn every_way_out_of_downed_ends_in_an_enable() {
    let mut rules = downed_rules();
    let mut lock = PlayerLock::new();
    assert_eq!(
        lock.step(rules.locks_controls(), true),
        Some(PlayerLockCall::Disable)
    );
    rules.on_partner_event(CombatEvent::GameOver);
    assert_eq!(
        lock.step(rules.locks_controls(), true),
        Some(PlayerLockCall::Enable)
    );
}

fn revive_progress_sends(effects: &[DownedEffect]) -> Vec<u8> {
    effects
        .iter()
        .filter_map(|effect| match effect {
            DownedEffect::Send(CombatEvent::ReviveProgress(percent)) => Some(*percent),
            _ => None,
        })
        .collect()
}

#[test]
fn holding_a_revive_sends_its_progress_a_few_times_a_second() {
    let mut rules = rules_with_downed_partner();
    let close = Some(1.0);
    let first = revive_progress_sends(&rules.step(step(FULL_HEALTH, close)));
    assert_eq!(first.len(), 1);
    assert!(first[0] > 0);
    let mut sends = 0;
    for _ in 0..4 {
        sends += revive_progress_sends(&rules.step(step(FULL_HEALTH, close))).len();
    }
    assert!((1..=4).contains(&sends));
}

#[test]
fn letting_go_of_a_revive_sends_zero_once() {
    let mut rules = rules_with_downed_partner();
    rules.step(step(FULL_HEALTH, Some(1.0)));
    let away = revive_progress_sends(&rules.step(step(FULL_HEALTH, None)));
    assert_eq!(away, vec![0]);
    assert!(revive_progress_sends(&rules.step(step(FULL_HEALTH, None))).is_empty());
}

#[test]
fn a_finished_revive_does_not_send_a_trailing_zero() {
    let mut rules = rules_with_downed_partner();
    rules.step(step(FULL_HEALTH, Some(1.0)));
    rules.request_revive_partner();
    assert!(revive_progress_sends(&rules.step(step(FULL_HEALTH, Some(1.0)))).is_empty());
}

#[test]
fn a_downed_player_sees_the_partners_revive_progress() {
    let mut rules = downed_rules();
    rules.on_partner_event(CombatEvent::ReviveProgress(50));
    assert_eq!(rules.incoming_revive_progress(), 0.5);
    rules.on_partner_event(CombatEvent::ReviveProgress(0));
    assert_eq!(rules.incoming_revive_progress(), 0.0);
}

#[test]
fn incoming_revive_progress_expires_when_the_messages_stop() {
    let mut rules = downed_rules();
    rules.on_partner_event(CombatEvent::ReviveProgress(60));
    let ticks = (INCOMING_REVIVE_PROGRESS_STALE_SECONDS / TICK_SECONDS).ceil() as usize;
    for _ in 0..ticks {
        rules.step(step(DOWNED_HEALTH, None));
    }
    assert_eq!(rules.incoming_revive_progress(), 0.0);
}

#[test]
fn incoming_revive_progress_is_ignored_while_up_and_cleared_on_revive() {
    let mut up = DownedRules::new();
    up.on_partner_event(CombatEvent::ReviveProgress(40));
    assert_eq!(up.incoming_revive_progress(), 0.0);
    let mut rules = downed_rules();
    rules.on_partner_event(CombatEvent::ReviveProgress(90));
    rules.on_partner_event(CombatEvent::Revived);
    assert_eq!(rules.incoming_revive_progress(), 0.0);
}

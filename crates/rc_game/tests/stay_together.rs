use rc_game::Position;
use rc_game::offsets::{ENTITY_ARRIVAL_POINT_OFFSET, ENTITY_RVA, ENTITY_ZONE_OFFSET};
use rc_game::stay_together::{
    ANNOUNCE_RESEND_MS, AnnounceConditions, DEFAULT_LEASH_UNITS, FOLLOW_RETRY_MS, FloorHit,
    FollowConditions, FollowRole, FollowState, FollowStep, GoToPartnerRefusal, HostAnnouncer,
    Leash, MAX_LEASH_UNITS, MIN_LEASH_UNITS, PARTNER_TIMEOUT_MS, PhaseTarget, SpotRefusal,
    TOGETHER_FREE_TO_ACT_GRACE_MS, WaitReason, WorldProbe, announced_target,
    check_partner_reachable, choose_spot, distance_between, local_phase_index,
};
use rc_game::{PHASE_TABLE, Replicant};

use crate::fake_memory::FakeMemory;

const TARGET: PhaseTarget = PhaseTarget { phase: 4, point: 2 };

fn guest_conditions() -> FollowConditions {
    FollowConditions {
        enabled: true,
        role: FollowRole::Guest,
        connected: true,
        free_to_act: true,
        local_phase: Some(1),
        now_ms: 10_000,
    }
}

fn host_conditions(current: Option<PhaseTarget>, now_ms: u64) -> AnnounceConditions {
    AnnounceConditions {
        role: FollowRole::Host,
        connected: true,
        current,
        now_ms,
    }
}

#[test]
fn only_slot_zero_is_the_host() {
    assert_eq!(FollowRole::from_slot(Some(0)), FollowRole::Host);
    assert_eq!(FollowRole::from_slot(Some(1)), FollowRole::Guest);
    assert_eq!(FollowRole::from_slot(None), FollowRole::Unknown);
}

#[test]
fn a_free_guest_goes_to_the_hosts_phase_and_point() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    assert_eq!(follow.step(&guest_conditions()), FollowStep::Go(TARGET));
}

#[test]
fn a_guest_who_is_not_free_waits_and_goes_once_free() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let busy = FollowConditions {
        free_to_act: false,
        ..guest_conditions()
    };
    assert_eq!(
        follow.step(&busy),
        FollowStep::Waiting(WaitReason::NotFreeToAct)
    );
    assert_eq!(follow.step(&guest_conditions()), FollowStep::Go(TARGET));
}

#[test]
fn a_guest_already_in_the_phase_does_nothing_and_forgets_the_request() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let there = FollowConditions {
        local_phase: Some(TARGET.phase),
        ..guest_conditions()
    };
    assert_eq!(follow.step(&there), FollowStep::Idle);
    assert_eq!(follow.pending(), None);
}

#[test]
fn an_unreadable_local_phase_waits() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let loading = FollowConditions {
        local_phase: None,
        ..guest_conditions()
    };
    assert_eq!(
        follow.step(&loading),
        FollowStep::Waiting(WaitReason::PhaseUnreadable)
    );
}

#[test]
fn a_failed_change_is_retried_after_the_cooldown_not_every_tick() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    assert_eq!(follow.step(&guest_conditions()), FollowStep::Go(TARGET));
    let soon = FollowConditions {
        now_ms: 10_000 + FOLLOW_RETRY_MS - 1,
        ..guest_conditions()
    };
    assert_eq!(
        follow.step(&soon),
        FollowStep::Waiting(WaitReason::RetryCooldown)
    );
    let later = FollowConditions {
        now_ms: 10_000 + FOLLOW_RETRY_MS,
        ..guest_conditions()
    };
    assert_eq!(follow.step(&later), FollowStep::Go(TARGET));
}

#[test]
fn a_newer_host_move_replaces_the_pending_one() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let newer = PhaseTarget { phase: 7, point: 0 };
    follow.host_moved_to(newer);
    assert_eq!(follow.step(&guest_conditions()), FollowStep::Go(newer));
}

#[test]
fn switching_following_off_drops_the_pending_request() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let off = FollowConditions {
        enabled: false,
        ..guest_conditions()
    };
    assert_eq!(follow.step(&off), FollowStep::Idle);
    assert_eq!(follow.step(&guest_conditions()), FollowStep::Idle);
}

#[test]
fn the_host_never_follows_and_a_disconnected_guest_does_not_either() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let host = FollowConditions {
        role: FollowRole::Host,
        ..guest_conditions()
    };
    assert_eq!(follow.step(&host), FollowStep::Idle);
    follow.host_moved_to(TARGET);
    let alone = FollowConditions {
        connected: false,
        ..guest_conditions()
    };
    assert_eq!(follow.step(&alone), FollowStep::Idle);
}

#[test]
fn the_host_announces_at_once_then_on_a_phase_change_or_the_resend_interval() {
    let mut announcer = HostAnnouncer::new();
    assert_eq!(
        announcer.poll(&host_conditions(Some(TARGET), 0)),
        Some(TARGET)
    );
    assert_eq!(announcer.poll(&host_conditions(Some(TARGET), 100)), None);
    let moved = PhaseTarget { phase: 9, point: 1 };
    assert_eq!(
        announcer.poll(&host_conditions(Some(moved), 200)),
        Some(moved)
    );
    assert_eq!(
        announcer.poll(&host_conditions(Some(moved), 200 + ANNOUNCE_RESEND_MS)),
        Some(moved)
    );
}

#[test]
fn the_host_announces_again_after_a_reconnect() {
    let mut announcer = HostAnnouncer::new();
    announcer.poll(&host_conditions(Some(TARGET), 0));
    let alone = AnnounceConditions {
        connected: false,
        ..host_conditions(Some(TARGET), 50)
    };
    assert_eq!(announcer.poll(&alone), None);
    assert_eq!(
        announcer.poll(&host_conditions(Some(TARGET), 100)),
        Some(TARGET)
    );
}

#[test]
fn a_guest_or_an_unreadable_host_phase_announces_nothing() {
    let mut announcer = HostAnnouncer::new();
    let guest = AnnounceConditions {
        role: FollowRole::Guest,
        ..host_conditions(Some(TARGET), 0)
    };
    assert_eq!(announcer.poll(&guest), None);
    assert_eq!(announcer.poll(&host_conditions(None, 0)), None);
}

#[test]
fn the_leash_is_exceeded_only_past_its_length() {
    let leash = Leash::new(30.0);
    assert!(!leash.is_exceeded_by(Some(30.0)));
    assert!(leash.is_exceeded_by(Some(30.1)));
    assert!(!leash.is_exceeded_by(None));
}

#[test]
fn the_leash_length_is_kept_in_range() {
    assert_eq!(Leash::new(0.0).units(), MIN_LEASH_UNITS);
    assert_eq!(Leash::new(1.0e9).units(), MAX_LEASH_UNITS);
    assert_eq!(Leash::new(f32::NAN).units(), DEFAULT_LEASH_UNITS);
}

#[test]
fn distance_is_none_for_a_garbage_position() {
    assert_eq!(
        distance_between([0.0, 0.0, 0.0], [3.0, 4.0, 0.0]),
        Some(5.0)
    );
    assert_eq!(distance_between([0.0; 3], [f32::NAN, 0.0, 0.0]), None);
}

#[test]
fn going_to_the_partner_needs_a_fresh_peer_in_the_same_phase() {
    assert_eq!(
        check_partner_reachable(true, Some(100), Some(2), Some(2)),
        Ok(())
    );
    assert_eq!(
        check_partner_reachable(false, Some(100), Some(2), Some(2)),
        Err(GoToPartnerRefusal::NotConnected)
    );
    assert_eq!(
        check_partner_reachable(true, None, Some(2), None),
        Err(GoToPartnerRefusal::NotConnected)
    );
    assert_eq!(
        check_partner_reachable(true, Some(PARTNER_TIMEOUT_MS + 1), Some(2), Some(2)),
        Err(GoToPartnerRefusal::PartnerSilent)
    );
    assert_eq!(
        check_partner_reachable(true, Some(100), None, Some(2)),
        Err(GoToPartnerRefusal::OwnPhaseUnknown)
    );
    assert_eq!(
        check_partner_reachable(true, Some(100), Some(1), Some(2)),
        Err(GoToPartnerRefusal::DifferentPhase)
    );
}

struct FlatWorld {
    floor_y: f32,
    normal_y: f32,
    wall: Option<(f32, f32, f32)>,
    drop_beyond_x: Option<f32>,
}

impl FlatWorld {
    fn flat() -> Self {
        Self {
            floor_y: 0.0,
            normal_y: 1.0,
            wall: None,
            drop_beyond_x: None,
        }
    }
}

impl WorldProbe for FlatWorld {
    fn floor_below(&self, x: f32, _y: f32, _z: f32) -> Option<FloorHit> {
        let y = match self.drop_beyond_x {
            Some(edge) if x > edge => self.floor_y - 5.0,
            _ => self.floor_y,
        };
        Some(FloorHit {
            y,
            normal_y: self.normal_y,
        })
    }

    fn body_fits(&self, x: f32, _y: f32, z: f32) -> bool {
        self.wall
            .is_none_or(|(wx, wz, radius)| (x - wx).hypot(z - wz) > radius)
    }
}

struct VoidWorld;

impl WorldProbe for VoidWorld {
    fn floor_below(&self, _x: f32, _y: f32, _z: f32) -> Option<FloorHit> {
        None
    }

    fn body_fits(&self, _x: f32, _y: f32, _z: f32) -> bool {
        true
    }
}

fn at(x: f32, z: f32) -> Position {
    Position { x, y: 0.0, z }
}

#[test]
fn the_spot_is_on_the_side_the_local_player_comes_from() {
    let spot = choose_spot(at(0.0, 0.0), at(50.0, 0.0), &FlatWorld::flat()).expect("flat ground");
    assert!(spot.x > 1.0 && spot.z.abs() < 0.01);
    assert_eq!(spot.y, 0.0);
}

#[test]
fn a_blocked_spot_is_skipped_for_the_next_free_one() {
    let world = FlatWorld {
        wall: Some((1.5, 0.0, 0.5)),
        ..FlatWorld::flat()
    };
    let spot = choose_spot(at(0.0, 0.0), at(50.0, 0.0), &world).expect("another side is free");
    assert!(spot.x.hypot(spot.z) >= 1.5 - 0.001);
    assert!((spot.x - 1.5).hypot(spot.z) > 0.5);
}

#[test]
fn a_spot_over_a_drop_is_never_chosen() {
    let world = FlatWorld {
        drop_beyond_x: Some(0.5),
        ..FlatWorld::flat()
    };
    let spot = choose_spot(at(0.0, 0.0), at(50.0, 0.0), &world).expect("the other side is flat");
    assert!(spot.x <= 0.5);
}

#[test]
fn steep_ground_is_refused() {
    let world = FlatWorld {
        normal_y: 0.3,
        ..FlatWorld::flat()
    };
    assert_eq!(
        choose_spot(at(0.0, 0.0), at(5.0, 0.0), &world),
        Err(SpotRefusal::NoSafeSpot)
    );
}

#[test]
fn no_ground_under_the_partner_is_refused_with_its_own_reason() {
    assert_eq!(
        choose_spot(at(0.0, 0.0), at(5.0, 0.0), &VoidWorld),
        Err(SpotRefusal::NoGroundUnderPartner)
    );
}

#[test]
fn a_spot_in_a_walled_in_pocket_is_refused() {
    let world = FlatWorld {
        wall: Some((0.0, 0.0, 10.0)),
        ..FlatWorld::flat()
    };
    assert_eq!(
        choose_spot(at(0.0, 0.0), at(5.0, 0.0), &world),
        Err(SpotRefusal::NoSafeSpot)
    );
}

fn memory_in_phase(name: &str, point: i32) -> FakeMemory {
    let mut memory = FakeMemory::new();
    let mut buffer = [0_u8; 0x20];
    buffer[..name.len()].copy_from_slice(name.as_bytes());
    memory.put_at_rva(ENTITY_RVA + ENTITY_ZONE_OFFSET, &buffer);
    memory.put_at_rva(
        ENTITY_RVA + ENTITY_ARRIVAL_POINT_OFFSET,
        &point.to_le_bytes(),
    );
    memory
}

#[test]
fn the_announced_target_is_the_phase_index_and_the_arrival_point() {
    let name = PHASE_TABLE[3].name;
    let game = Replicant::new(memory_in_phase(name, 6));
    assert_eq!(local_phase_index(&game), Some(3));
    assert_eq!(
        announced_target(&game),
        Some(PhaseTarget { phase: 3, point: 6 })
    );
}

#[test]
fn an_arrival_point_outside_the_valid_range_becomes_point_zero() {
    let name = PHASE_TABLE[3].name;
    let game = Replicant::new(memory_in_phase(name, -1));
    assert_eq!(
        announced_target(&game),
        Some(PhaseTarget { phase: 3, point: 0 })
    );
}

#[test]
fn a_phase_name_outside_the_table_announces_nothing() {
    let game = Replicant::new(memory_in_phase("NOT_A_PHASE", 1));
    assert_eq!(announced_target(&game), None);
}

#[test]
fn a_guest_who_stays_busy_goes_anyway_after_the_grace() {
    let mut follow = FollowState::new();
    follow.host_moved_to(TARGET);
    let busy = |now_ms| FollowConditions {
        free_to_act: false,
        now_ms,
        ..guest_conditions()
    };
    assert_eq!(
        follow.step(&busy(10_000)),
        FollowStep::Waiting(WaitReason::NotFreeToAct)
    );
    assert_eq!(
        follow.step(&busy(10_000 + TOGETHER_FREE_TO_ACT_GRACE_MS)),
        FollowStep::Go(TARGET)
    );
}

#[test]
fn the_pause_menu_does_not_freeze_the_world_with_a_partner_but_keeps_input_blocked() {
    use rc_game::coop_pause::{PauseView, effective_control_mask};
    let open = PauseView {
        partner_connected: true,
        menu_open: true,
        event_scene: false,
    };
    assert_eq!(effective_control_mask([0x19F, 0, 0, 0], open), 0x180);
    assert_eq!(effective_control_mask([0x11F, 0, 0, 0], open), 0x180);
    let solo = PauseView {
        partner_connected: false,
        ..open
    };
    assert_eq!(effective_control_mask([0x19F, 0, 0, 0], solo), 0x19F);
    let cutscene = PauseView {
        event_scene: true,
        ..open
    };
    assert_eq!(effective_control_mask([0x19F, 0, 0, 0], cutscene), 0x19F);
}

#[test]
fn a_script_pause_without_a_menu_keeps_the_whole_mask() {
    use rc_game::coop_pause::{PauseView, effective_control_mask};
    let script_pause = PauseView {
        partner_connected: true,
        menu_open: false,
        event_scene: false,
    };
    assert_eq!(
        effective_control_mask([0x7FF, 0, 0, 0], script_pause),
        0x7FF
    );
}

#[test]
fn no_pause_softens_only_the_menu_slot() {
    use rc_game::coop_pause::{PauseView, effective_control_mask};
    let open = PauseView {
        partner_connected: true,
        menu_open: true,
        event_scene: false,
    };
    assert_eq!(effective_control_mask([0x19F, 0xA0, 0, 0], open), 0x1A0);
    assert_eq!(effective_control_mask([0x19F, 0, 0, 0x3FFF], open), 0x3FFF);
    assert_eq!(effective_control_mask([0x100, 0, 0, 0], open), 0x100);
}

#[test]
fn the_host_sends_its_fade_out_only_after_an_allowed_zone_change_and_the_fade_in_after_it() {
    use rc_game::fade_sync::{HostFadeSync, ZONE_CHANGE_FADE_WINDOW_MS};
    let mut sync = HostFadeSync::new();
    assert!(!sync.should_send_fade_out(1_000));
    assert!(!sync.should_send_fade_in());
    sync.arm(2_000);
    assert!(sync.should_send_fade_out(2_500));
    assert!(!sync.should_send_fade_out(2_600));
    assert!(sync.should_send_fade_in());
    assert!(!sync.should_send_fade_in());
    sync.arm(10_000);
    assert!(!sync.should_send_fade_out(10_000 + ZONE_CHANGE_FADE_WINDOW_MS));
}

#[test]
fn the_guest_fades_in_by_itself_when_the_hosts_fade_in_never_arrives() {
    use rc_game::fade_sync::{GUEST_FADE_BACKSTOP_MS, GuestFadeState};
    let mut guest = GuestFadeState::new();
    assert!(!guest.backstop_due(0));
    guest.started_fade_out(1_000);
    assert!(guest.holds_fade());
    assert!(!guest.backstop_due(1_000 + GUEST_FADE_BACKSTOP_MS - 1));
    assert!(guest.backstop_due(1_000 + GUEST_FADE_BACKSTOP_MS));
    assert!(!guest.holds_fade());
    guest.started_fade_out(5);
    guest.started_fade_in();
    assert!(!guest.backstop_due(100_000));
}

#[test]
fn the_fade_watch_reports_a_start_and_a_finish_once() {
    use rc_game::fade_sync::{FadeEdge, FadeWatch};
    let mut watch = FadeWatch::new();
    assert_eq!(watch.observe(false), FadeEdge::None);
    assert_eq!(watch.observe(true), FadeEdge::Started);
    assert_eq!(watch.observe(true), FadeEdge::None);
    assert_eq!(watch.observe(false), FadeEdge::Finished);
}

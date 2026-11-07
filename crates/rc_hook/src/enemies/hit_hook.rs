use core::ffi::c_void;
use core::ptr;
use std::collections::HashMap;
use std::panic;
use std::sync::atomic::{AtomicPtr, AtomicU32, Ordering};
use std::sync::{LazyLock, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use rc_game::GameMemory;
use rc_game::effects::bullet_owner;
use rc_game::boss_messages::held_hp_to_restore;
use rc_game::enemies::{
    AttackRecord, GUEST_HIT_REPEAT_WINDOW_MS, PlayerHitGuard, restored_time_scale, scaled_hp_damage,
};
use rc_game::offsets::{
    ACTOR_TIME_SCALE_OFFSET, ATTACK_FLAG_MAGIC, ATTACK_RECORD_HP_DAMAGE_OFFSET, RECEIVE_ATTACK_QUERY_FUNCTION_RVA,
};
use rc_proto::{EnemyHit, EnemyId, MAX_HIT_DAMAGE, Message, PlayerHit};

use crate::boss_messages::group_has_registered_life_callback;
use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::EnemySyncRole;
use crate::enemies::combat_view::{CombatView, applying_remote, with_view};
use crate::enemies::counters::{HookCount, count};
use crate::enemies::enemy_id_text;
use crate::enemies::game_calls::EnemyCalls;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;
use crate::story_relay::shared_story_events;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_RECEIVE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static LAST_LOGGED_MAGIC_HIT: AtomicU32 = AtomicU32::new(0);
static BODY_SWINGS: LazyLock<Mutex<PlayerHitGuard>> = LazyLock::new(Mutex::default);
static BODY_SWING_CLOCK: LazyLock<Instant> = LazyLock::new(Instant::now);
static REPORTED_SWINGS: LazyLock<Mutex<HashMap<(EnemyId, u32), Instant>>> =
    LazyLock::new(Mutex::default);

type ReceiveAttackQuery =
    unsafe extern "system" fn(*mut c_void, *mut c_void, *mut c_void, i32, u8) -> u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HitRoute {
    Run,
    Drop,
}

pub(super) fn install(build: VerifiedBuild) {
    if HOOKED_BUILD.set(build).is_err() {
        return;
    }
    #[expect(
        unsafe_code,
        reason = "hooks CActor::ReceiveAttackQuery by its address"
    )]
    // SAFETY: the RVA starts the base vslot 0x5D8 body every actor class uses (victim, attack
    // record, result, part index, force flag) in the verified build; the replacement has the
    // same five arguments and passes them on unchanged.
    unsafe {
        hook_address(
            "CActor::ReceiveAttackQuery",
            (build.module_base() + u64::from(RECEIVE_ATTACK_QUERY_FUNCTION_RVA)) as usize,
            receive_attack_query_detour as *const () as *mut c_void,
            &ORIGINAL_RECEIVE,
        );
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original ReceiveAttackQuery through the trampoline MinHook built"
)]
extern "system" fn receive_attack_query_detour(
    victim: *mut c_void,
    record: *mut c_void,
    result: *mut c_void,
    part: i32,
    force: u8,
) -> u64 {
    let route = if applying_remote() {
        HitRoute::Run
    } else {
        panic::catch_unwind(|| route_hit(victim as u64, record as u64)).unwrap_or(HitRoute::Run)
    };
    if route == HitRoute::Drop {
        return 0;
    }
    let original = ORIGINAL_RECEIVE.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    let scale_before = panic::catch_unwind(|| struck_copy_time_scale(victim as u64))
        .ok()
        .flatten();
    let hp_before = if applying_remote() {
        None
    } else {
        panic::catch_unwind(|| struck_copy_hp(victim as u64))
            .ok()
            .flatten()
    };
    // SAFETY: the trampoline continues into the original body, which has this exact signature.
    let outcome = unsafe {
        core::mem::transmute::<*mut c_void, ReceiveAttackQuery>(original)(
            victim, record, result, part, force,
        )
    };
    if let Some(before) = scale_before {
        let _ = panic::catch_unwind(|| restore_time_scale(victim as u64, before));
    }
    if let Some(before) = hp_before {
        let _ = panic::catch_unwind(|| restore_hp(victim as u64, before));
    }
    outcome
}

fn struck_copy_hp(victim: u64) -> Option<i32> {
    let build = HOOKED_BUILD.get()?;
    if !shared_story_events() {
        return None;
    }
    let copy_group = with_view(|view| {
        if view.role == Some(EnemySyncRole::Guest) {
            view.enemies.get(&victim).map(|id| id.group)
        } else {
            None
        }
    })?;
    if !group_has_registered_life_callback(build, copy_group) {
        return None;
    }
    let memory = InProcessMemory::new(build);
    EnemyCalls::new(build)
        .hp_of(&memory, victim)
        .map(|(hp, _)| hp)
}

fn restore_hp(victim: u64, before: i32) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let memory = InProcessMemory::new(build);
    let calls = EnemyCalls::new(build);
    let Some((after, _)) = calls.hp_of(&memory, victim) else {
        return;
    };
    let Some(target) = held_hp_to_restore(before, after) else {
        return;
    };
    // The host's HP snapshots own a held copy's HP; a local hit must not trip a life threshold early.
    if calls.set_hp(&memory, victim, target) {
        log_line(&format!(
            "combat: a hit lowered the hp of your copy at {victim:#x} from {before} to {after}; restored to {target}"
        ));
    }
}

fn struck_copy_time_scale(victim: u64) -> Option<f32> {
    let build = HOOKED_BUILD.get()?;
    let is_guest_copy = with_view(|view| {
        view.role == Some(EnemySyncRole::Guest) && view.enemies.contains_key(&victim)
    });
    if !is_guest_copy {
        return None;
    }
    InProcessMemory::new(build)
        .read_f32(victim + ACTOR_TIME_SCALE_OFFSET)
        .ok()
}

fn restore_time_scale(victim: u64, before: f32) {
    let Some(build) = HOOKED_BUILD.get() else {
        return;
    };
    let mut memory = InProcessMemory::new(build);
    let Ok(after) = memory.read_f32(victim + ACTOR_TIME_SCALE_OFFSET) else {
        return;
    };
    let Some(target) = restored_time_scale(before, after) else {
        return;
    };
    // A held copy never runs the state step that ends the hit's slow-down, so its motion would stay frozen.
    if memory
        .write_f32(victim + ACTOR_TIME_SCALE_OFFSET, target)
        .is_ok()
    {
        log_line(&format!(
            "combat: a hit changed the time scale of your copy at {victim:#x} from {before:.2} to {after:.2}; restored to {target:.2}"
        ));
    }
}

fn route_hit(victim: u64, record: u64) -> HitRoute {
    let Some(build) = HOOKED_BUILD.get() else {
        return HitRoute::Run;
    };
    with_view(|view| match view.role {
        None => HitRoute::Run,
        Some(EnemySyncRole::Guest) => guest_route(build, view, victim, record),
        Some(EnemySyncRole::Host) => {
            host_watch(build, view, victim, record);
            HitRoute::Run
        }
    })
}

fn guest_route(build: &VerifiedBuild, view: &CombatView, victim: u64, record: u64) -> HitRoute {
    let victim_copy = view.enemies.get(&victim).copied();
    if victim_copy.is_none() && victim != view.local_player {
        return HitRoute::Run;
    }
    let memory = InProcessMemory::new(build);
    let Some(attack) = AttackRecord::read(&memory, record) else {
        return HitRoute::Run;
    };
    let sources = attack.sources(&memory);
    if let Some(id) = victim_copy {
        let bullet_of_local = sources[0].and_then(|attacker| bullet_owner(&memory, attacker));
        log_magic_hit(&attack, sources, bullet_of_local, view.local_player);
        let fired_by_local = sources.contains(&Some(view.local_player))
            || bullet_of_local == Some(view.local_player);
        if view.local_player != 0 && fired_by_local {
            report_hit(view.phase, id, &attack);
        }
        return HitRoute::Run;
    }
    // The host decides every enemy hit on this player; a held copy's own swing never lands.
    if let Some(attacker) = sources.iter().find_map(|source| view.is_enemy(*source)) {
        count(HookCount::LocalEnemyHitBlocked);
        log_line(&format!(
            "combat: blocked {}'s local hit on you (hp {}, kind {}); the host sends enemy hits",
            enemy_id_text(attacker),
            attack.hp_damage,
            attack.attack_kind
        ));
        return HitRoute::Drop;
    }
    HitRoute::Run
}

fn log_magic_hit(
    attack: &AttackRecord,
    sources: [Option<u64>; 2],
    bullet_of_local: Option<u64>,
    local_player: u64,
) {
    if attack.flags & ATTACK_FLAG_MAGIC == 0
        || LAST_LOGGED_MAGIC_HIT.swap(attack.hit_id, Ordering::Relaxed) == attack.hit_id
    {
        return;
    }
    log_line(&format!(
        "combat: magic hit {}: source class {}, attacker found {}, owner is you {}, bullet owner is you {}",
        attack.hit_id,
        attack.source_class,
        sources[0].is_some(),
        sources[1] == Some(local_player),
        bullet_of_local == Some(local_player)
    ));
}

fn host_watch(build: &VerifiedBuild, view: &CombatView, victim: u64, record: u64) {
    if view.local_player != 0 && victim == view.local_player {
        scale_enemy_hit_on_host(build, view, record);
        return;
    }
    if view.puppet == 0 || victim != view.puppet {
        return;
    }
    let memory = InProcessMemory::new(build);
    let Some(attack) = AttackRecord::read(&memory, record) else {
        return;
    };
    let Some(attacker) = attack
        .sources(&memory)
        .iter()
        .find_map(|source| view.is_enemy(*source))
    else {
        return;
    };
    let Some(net) = net::handle() else {
        return;
    };
    let now_ms = u64::try_from(BODY_SWING_CLOCK.elapsed().as_millis()).unwrap_or(u64::MAX);
    let counts = BODY_SWINGS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .counts(now_ms, attacker, attack.hit_id);
    if !counts {
        return;
    }
    let hit = PlayerHit {
        phase: view.phase,
        attacker,
        hp_damage: damage_on_wire(scaled_hp_damage(
            attack.hp_damage,
            view.enemy_damage_bonus_percent,
        )),
        flags: attack.flags,
        attack_kind: attack.attack_kind.cast_unsigned(),
        hit_id: attack.hit_id,
    };
    net.send_relayed(Message::PlayerHit(hit));
    count(HookCount::PlayerHitSent);
    log_line(&format!(
        "combat: {} hit the guest's body: hp {} (was {}, +{}%), flags {:#x}, kind {}, hit id {} (sent)",
        enemy_id_text(attacker),
        hit.hp_damage,
        attack.hp_damage,
        view.enemy_damage_bonus_percent,
        hit.flags,
        hit.attack_kind,
        attack.hit_id
    ));
}

fn scale_enemy_hit_on_host(build: &VerifiedBuild, view: &CombatView, record: u64) {
    if view.enemy_damage_bonus_percent == 0 {
        return;
    }
    let mut memory = InProcessMemory::new(build);
    let Some(attack) = AttackRecord::read(&memory, record) else {
        return;
    };
    let Some(attacker) = attack
        .sources(&memory)
        .iter()
        .find_map(|source| view.is_enemy(*source))
    else {
        return;
    };
    let scaled = scaled_hp_damage(attack.hp_damage, view.enemy_damage_bonus_percent);
    if scaled == attack.hp_damage
        || memory
            .write_i32(record + ATTACK_RECORD_HP_DAMAGE_OFFSET, scaled)
            .is_err()
    {
        return;
    }
    log_line(&format!(
        "difficulty: {} hit you for hp {scaled} (was {}, +{}%), hit id {}",
        enemy_id_text(attacker),
        attack.hp_damage,
        view.enemy_damage_bonus_percent,
        attack.hit_id
    ));
}

fn first_report_of_swing(id: EnemyId, hit_id: u32) -> bool {
    // The game hands the same swing to a held copy every frame it overlaps; hit id 0 is no swing.
    if hit_id == 0 {
        return true;
    }
    let now = Instant::now();
    let window = Duration::from_millis(GUEST_HIT_REPEAT_WINDOW_MS);
    let mut reported = REPORTED_SWINGS
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    reported.retain(|_, at| now.saturating_duration_since(*at) < window);
    reported.insert((id, hit_id), now).is_none()
}

fn report_hit(phase: u16, id: EnemyId, attack: &AttackRecord) {
    if !first_report_of_swing(id, attack.hit_id) {
        return;
    }
    let Some(net) = net::handle() else {
        return;
    };
    let hit = EnemyHit {
        phase,
        id,
        hit_id: attack.hit_id,
        hp_damage: damage_on_wire(attack.hp_damage),
        second_gauge: damage_on_wire(attack.second_gauge),
        third_gauge: damage_on_wire(attack.third_gauge),
        flags: attack.flags,
        attack_kind: attack.attack_kind.cast_unsigned(),
    };
    net.send_relayed(Message::EnemyHit(hit));
    count(HookCount::HitSent);
    log_line(&format!(
        "combat: you hit {}: hp {}, gauges {}/{}, flags {:#x}, kind {}, source {}, hit id {} (sent)",
        enemy_id_text(id),
        hit.hp_damage,
        hit.second_gauge,
        hit.third_gauge,
        hit.flags,
        hit.attack_kind,
        attack.source_class,
        hit.hit_id
    ));
}

fn damage_on_wire(damage: i32) -> u32 {
    damage
        .clamp(0, MAX_HIT_DAMAGE.cast_signed())
        .cast_unsigned()
}

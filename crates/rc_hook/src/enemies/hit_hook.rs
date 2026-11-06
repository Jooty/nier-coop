use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicPtr, Ordering};

use rc_game::GameMemory;
use rc_game::enemies::{AttackRecord, scaled_hp_damage};
use rc_game::offsets::{ATTACK_RECORD_HP_DAMAGE_OFFSET, RECEIVE_ATTACK_QUERY_FUNCTION_RVA};
use rc_proto::{EnemyHit, EnemyId, MAX_HIT_DAMAGE, Message, PlayerHit};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::enemies::EnemySyncRole;
use crate::enemies::combat_view::{CombatView, applying_remote, with_view};
use crate::enemies::counters::{HookCount, count};
use crate::enemies::enemy_id_text;
use crate::game_hooks::hook_address;
use crate::memory::InProcessMemory;
use crate::net;

static HOOKED_BUILD: OnceLock<VerifiedBuild> = OnceLock::new();
static ORIGINAL_RECEIVE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

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
    // SAFETY: the trampoline continues into the original body, which has this exact signature.
    unsafe {
        core::mem::transmute::<*mut c_void, ReceiveAttackQuery>(original)(
            victim, record, result, part, force,
        )
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
        if view.local_player != 0 && sources.contains(&Some(view.local_player)) {
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
    let hit = PlayerHit {
        phase: view.phase,
        attacker,
        hp_damage: damage_on_wire(scaled_hp_damage(
            attack.hp_damage,
            view.enemy_damage_bonus_percent,
        )),
        flags: attack.flags,
        attack_kind: attack.attack_kind.cast_unsigned(),
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

fn report_hit(phase: u16, id: EnemyId, attack: &AttackRecord) {
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

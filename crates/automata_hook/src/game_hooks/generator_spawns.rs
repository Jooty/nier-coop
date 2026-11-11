use core::cell::Cell;
use core::ffi::c_void;
use core::ptr;
use std::collections::BTreeSet;
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, Ordering};

use automata_game::address::{
    GENERATOR_LEVEL_PICK_FUNCTION, GENERATOR_SLOT_ADD_FUNCTION, GENERATOR_SLOT_FREE_FUNCTION,
    GENERATOR_SLOT_STEP_FUNCTION, GENERATOR_SPAWNER_UPDATE_FUNCTION, GameAddress,
};
use automata_game::enemies::{
    EnemySpawn, GENERATOR_ACTION_ID_OFFSET, GENERATOR_LEVEL_PICK_FUNCTION_START,
    GENERATOR_SCREEN_TEST_KIND_OFFSET, GENERATOR_SLOT_ADD_FUNCTION_START,
    GENERATOR_SLOT_FREE_FUNCTION_START, GENERATOR_SLOT_LEN,
    GENERATOR_SLOT_STEP_FUNCTION_START_IN_THE_COMPAT_EXE,
    GENERATOR_SLOT_STEP_FUNCTION_START_IN_THE_MAIN_EXE, GENERATOR_SPAWNER_OFFSET,
    GENERATOR_SPAWNER_UPDATE_FUNCTION_START, GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET,
    GeneratorHeader, GeneratorSlot, GuestGeneratorStep, GuestGenerators, ObservedSpawn,
    SlotRemoval, generator_slots,
};
use automata_game::partner::code_starts_with;
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;

const SPAWN_HOOK_LINES_LOGGED_AT_MOST: u32 = 400;
const MOST_OBSERVED_SPAWNS_KEPT_BETWEEN_TICKS: usize = 256;
const NO_SCREEN_TEST: i32 = 0;
const GENERATOR_STAYS_IN_ITS_RUN: u64 = 1;
const GENERATOR_RUN_IS_OVER: u64 = 0;
const SPAWNING_ALLOWED: u8 = 1;
const SPAWNING_STOPPED: u8 = 0;

static ORIGINAL_SPAWNER_UPDATE: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_SLOT_STEP: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static ORIGINAL_LEVEL_PICK: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());
static WATCH: Mutex<Option<SpawnWatch>> = Mutex::new(None);

thread_local! {
    static LEVEL_THE_GAME_PICKED_IN_THIS_STEP: Cell<Option<i32>> = const { Cell::new(None) };
    static LEVEL_OF_THE_HOSTS_SPAWN_IN_THIS_STEP: Cell<Option<i32>> = const { Cell::new(None) };
}

type SpawnerUpdate = unsafe extern "system" fn(usize, usize, u8) -> u64;
type SlotStep = unsafe extern "system" fn(usize, usize, u8);
type LevelPick = unsafe extern "system" fn(usize, usize) -> i32;

struct SpawnWatch {
    calls: GameCalls,
    guest_generators_are_held: bool,
    guest: GuestGenerators,
    observed: Vec<ObservedSpawn>,
    generators_left_to_the_game: BTreeSet<u32>,
    generators_said_to_be_held: BTreeSet<u32>,
    lines_left: u32,
}

struct SlotBeforeItsStep {
    calls: GameCalls,
    generator_id: u32,
    slot: GeneratorSlot,
    generator_is_held: bool,
    hosts_spawn: Option<EnemySpawn>,
}

struct HeldUpdate {
    calls: GameCalls,
    generator_id: u32,
    header: GeneratorHeader,
    slots: Vec<GeneratorSlot>,
    step: GuestGeneratorStep,
}

/// Hooks the generator's spawner update, its slot step and its level pick; false, with every
/// generator left to its own game, when a function start differs from the noted one.
pub fn install_generator_spawns(calls: GameCalls) -> bool {
    let starts_match = start_is_a_noted_one(
        &calls,
        GENERATOR_SPAWNER_UPDATE_FUNCTION,
        &[&GENERATOR_SPAWNER_UPDATE_FUNCTION_START],
    ) && start_is_a_noted_one(
        &calls,
        GENERATOR_SLOT_STEP_FUNCTION,
        &[
            &GENERATOR_SLOT_STEP_FUNCTION_START_IN_THE_MAIN_EXE,
            &GENERATOR_SLOT_STEP_FUNCTION_START_IN_THE_COMPAT_EXE,
        ],
    ) && start_is_a_noted_one(
        &calls,
        GENERATOR_LEVEL_PICK_FUNCTION,
        &[&GENERATOR_LEVEL_PICK_FUNCTION_START],
    ) && start_is_a_noted_one(
        &calls,
        GENERATOR_SLOT_FREE_FUNCTION,
        &[&GENERATOR_SLOT_FREE_FUNCTION_START],
    ) && start_is_a_noted_one(
        &calls,
        GENERATOR_SLOT_ADD_FUNCTION,
        &[&GENERATOR_SLOT_ADD_FUNCTION_START],
    );
    if !starts_match {
        log_line(
            "enemies: a generator function does not start as noted, generators are not hooked and every copy keeps its own spawns",
        );
        return false;
    }
    let Ok(mut watch) = WATCH.lock() else {
        return false;
    };
    *watch = Some(SpawnWatch {
        calls,
        guest_generators_are_held: false,
        guest: GuestGenerators::default(),
        observed: Vec::new(),
        generators_left_to_the_game: BTreeSet::new(),
        generators_said_to_be_held: BTreeSet::new(),
        lines_left: SPAWN_HOOK_LINES_LOGGED_AT_MOST,
    });
    drop(watch);
    hook(
        "generator level pick (the host's level for a spawn)",
        &calls,
        GENERATOR_LEVEL_PICK_FUNCTION,
        level_pick_detour as *const (),
        &ORIGINAL_LEVEL_PICK,
    ) && hook(
        "generator slot step (each spawn a generator makes)",
        &calls,
        GENERATOR_SLOT_STEP_FUNCTION,
        slot_step_detour as *const (),
        &ORIGINAL_SLOT_STEP,
    ) && hook(
        "generator spawner update (the guest's generators wait for the host)",
        &calls,
        GENERATOR_SPAWNER_UPDATE_FUNCTION,
        spawner_update_detour as *const (),
        &ORIGINAL_SPAWNER_UPDATE,
    )
}

/// While true, this copy's generators decide nothing: their slots come from the host's spawns.
pub fn set_guest_generators_held(held: bool) {
    if let Ok(mut watch) = WATCH.lock()
        && let Some(watch) = watch.as_mut()
    {
        watch.guest_generators_are_held = held;
        if !held {
            watch.generators_said_to_be_held.clear();
        }
    }
}

/// The generators no id names: they are never held and their spawns are never told.
pub fn set_generators_left_to_the_game(ids: BTreeSet<u32>) {
    if let Ok(mut watch) = WATCH.lock()
        && let Some(watch) = watch.as_mut()
    {
        watch.generators_left_to_the_game = ids;
    }
}

/// The spawns this copy's generators decided themselves since the last call.
pub fn take_observed_spawns() -> Vec<ObservedSpawn> {
    WATCH
        .lock()
        .ok()
        .and_then(|mut watch| {
            watch
                .as_mut()
                .map(|watch| core::mem::take(&mut watch.observed))
        })
        .unwrap_or_default()
}

pub fn with_guest_generators<R>(act: impl FnOnce(&mut GuestGenerators) -> R) -> Option<R> {
    let mut watch = WATCH.lock().ok()?;
    watch.as_mut().map(|watch| act(&mut watch.guest))
}

#[expect(
    unsafe_code,
    reason = "calls the original spawner update through the trampoline MinHook built, and the game's slot list functions"
)]
extern "system" fn spawner_update_detour(spawner: usize, generator: usize, may_spawn: u8) -> u64 {
    let original = ORIGINAL_SPAWNER_UPDATE.load(Ordering::Acquire);
    if original.is_null() {
        return GENERATOR_STAYS_IN_ITS_RUN;
    }
    // SAFETY: the trampoline continues into the original function.
    let original = unsafe { core::mem::transmute::<*mut c_void, SpawnerUpdate>(original) };
    let Some(held) = held_generator_before_its_update(spawner, generator) else {
        // SAFETY: the arguments are the ones received.
        return unsafe { original(spawner, generator, may_spawn) };
    };
    let calls = held.calls;
    for (index, removal) in &held.step.removals {
        // SAFETY: the slot was read from this generator's own list a moment ago, inside the
        // game's update of the generator, and nothing has changed the list since.
        unsafe { remove_slot(&held, *index, *removal) };
    }
    for spawn in &held.step.adds {
        // SAFETY: the generator is the one the game is updating right now.
        unsafe { add_slot_for(&held, generator, spawn) };
    }
    let count_at = spawner as u64;
    let screen_test_at = (generator + GENERATOR_SCREEN_TEST_KIND_OFFSET) as u64;
    let own_count = calls.read_u64_at(count_at).ok();
    // With the count at the run's total the game's own code decides no spawn in this call.
    let run_is_full = u64::try_from(held.header.enemies_of_one_run).unwrap_or(0);
    let holds_by_the_count =
        own_count.is_some() && calls.write_u64_at(count_at, run_is_full).is_ok();
    if !holds_by_the_count {
        say("enemies: a held generator's count cannot be written, it runs with the game's own stop-spawn flag instead".to_owned());
        // SAFETY: the arguments are the ones received, with the game's own flag for no new spawns.
        unsafe { original(spawner, generator, SPAWNING_STOPPED) };
        return GENERATOR_STAYS_IN_ITS_RUN;
    }
    // The host decided these spawns: this copy's camera does not hold them back.
    let screen_test_is_off = calls.write_i32_at(screen_test_at, NO_SCREEN_TEST).is_ok();
    // SAFETY: the generator and its spawner are the ones received; spawning is allowed so the
    // slots made from the host's spawns get their enemies.
    unsafe { original(spawner, generator, SPAWNING_ALLOWED) };
    if screen_test_is_off {
        let _ = calls.write_i32_at(screen_test_at, held.header.screen_test_kind);
    }
    let count_after = held.step.spawned.map(u64::from).or(own_count).unwrap_or(0);
    let _ = calls.write_u64_at(count_at, count_after);
    note_slots_after_the_update(&held, generator);
    if held.step.finishes {
        say(format!(
            "enemies: generator {:#X} ends its run here because it has ended on the host",
            held.generator_id
        ));
        GENERATOR_RUN_IS_OVER
    } else {
        GENERATOR_STAYS_IN_ITS_RUN
    }
}

#[expect(
    unsafe_code,
    reason = "calls the original slot step through the trampoline MinHook built"
)]
extern "system" fn slot_step_detour(slot: usize, generator: usize, may_spawn: u8) {
    let original = ORIGINAL_SLOT_STEP.load(Ordering::Acquire);
    if original.is_null() {
        return;
    }
    // SAFETY: the trampoline continues into the original function.
    let original = unsafe { core::mem::transmute::<*mut c_void, SlotStep>(original) };
    let before = slot_before_its_step(slot, generator);
    let Some((before, spawn)) = before
        .as_ref()
        .and_then(|before| Some((before, before.hosts_spawn?)))
    else {
        LEVEL_THE_GAME_PICKED_IN_THIS_STEP.set(None);
        // SAFETY: the arguments are the ones received.
        unsafe { original(slot, generator, may_spawn) };
        if let Some(before) = before.filter(|before| !before.generator_is_held) {
            note_a_spawn_of_this_copys_own(slot, generator, &before);
        }
        return;
    };
    let flag_at = (generator + GENERATOR_USES_THE_SECOND_SET_VALUES_OFFSET) as u64;
    LEVEL_OF_THE_HOSTS_SPAWN_IN_THIS_STEP.set(spawn.level);
    // The enemy gets the set values the host's generator used, whichever this copy's would use.
    let own_flag = before.calls.read_u8_at(flag_at).ok().filter(|_| {
        before
            .calls
            .write_u8_at(flag_at, u8::from(spawn.uses_the_second_set_values))
            .is_ok()
    });
    // SAFETY: the arguments are the ones received.
    unsafe { original(slot, generator, may_spawn) };
    if let Some(own_flag) = own_flag {
        let _ = before.calls.write_u8_at(flag_at, own_flag);
    }
    LEVEL_OF_THE_HOSTS_SPAWN_IN_THIS_STEP.set(None);
}

#[expect(
    unsafe_code,
    reason = "calls the original level pick through the trampoline MinHook built"
)]
extern "system" fn level_pick_detour(generator: usize, set: usize) -> i32 {
    if let Some(level) = LEVEL_OF_THE_HOSTS_SPAWN_IN_THIS_STEP.get() {
        return level;
    }
    let original = ORIGINAL_LEVEL_PICK.load(Ordering::Acquire);
    if original.is_null() {
        return 0;
    }
    // SAFETY: the trampoline continues into the original function with the arguments received.
    let level = unsafe { core::mem::transmute::<*mut c_void, LevelPick>(original)(generator, set) };
    LEVEL_THE_GAME_PICKED_IN_THIS_STEP.set(Some(level));
    level
}

fn start_is_a_noted_one(calls: &GameCalls, function: GameAddress, noted_starts: &[&[u8]]) -> bool {
    noted_starts.iter().any(|noted_start| {
        calls
            .read_bytes(function, noted_start.len())
            .is_ok_and(|found| code_starts_with(&found, noted_start))
    })
}

fn hook(
    label: &str,
    calls: &GameCalls,
    function: GameAddress,
    replacement: *const (),
    original: &AtomicPtr<c_void>,
) -> bool {
    #[expect(
        unsafe_code,
        reason = "hooks a generator function; each replacement has the signature of its original"
    )]
    // SAFETY: the address is a generator function of the verified build whose start was just
    // checked, and the replacement given for it takes and returns what the original does.
    unsafe {
        nc_hook::detour::hook_address(
            label,
            calls.address_value(function) as usize,
            replacement as *mut c_void,
            original,
        )
    }
}

fn held_generator_before_its_update(spawner: usize, generator: usize) -> Option<HeldUpdate> {
    if spawner != generator + GENERATOR_SPAWNER_OFFSET {
        return None;
    }
    let mut guard = WATCH.lock().ok()?;
    let watch = guard.as_mut()?;
    if !watch.guest_generators_are_held {
        return None;
    }
    let calls = watch.calls;
    let header = calls.read_generator(generator)?;
    let generator_id = header.action_id;
    if watch.generators_left_to_the_game.contains(&generator_id) {
        return None;
    }
    let slots = calls.read_generator_slots(&header);
    let step = watch.guest.step(generator_id, &slots);
    if watch.generators_said_to_be_held.insert(generator_id) {
        let line = format!(
            "enemies: spawn held: generator {generator_id:#X} runs here and decides nothing, the host's spawns are used (its own run would be {} enemies, {} slots it filled itself are taken out, the host's word on it: {})",
            header.enemies_of_one_run,
            step.removals
                .iter()
                .filter(|(_, removal)| *removal == SlotRemoval::NotTheHosts)
                .count(),
            watch
                .guest
                .host_state(generator_id)
                .map_or("none yet".to_owned(), |state| state.label())
        );
        say_within(watch, line);
    }
    Some(HeldUpdate {
        calls,
        generator_id,
        header,
        slots,
        step,
    })
}

#[expect(
    unsafe_code,
    reason = "calls the game's entity terminate and slot free on a slot of the generator being updated"
)]
unsafe fn remove_slot(held: &HeldUpdate, index: usize, removal: SlotRemoval) {
    let Some(slot) = held.slots.get(index) else {
        return;
    };
    let entity = if slot.holds_a_made_enemy() {
        held.calls.entity_of_handle(slot.handle)
    } else {
        0
    };
    // SAFETY: the entity was just resolved from the slot's handle, and the slot address is the
    // slot's place in the list read at the start of this update; this is what the game's own
    // clearing of a generator does to each slot.
    unsafe {
        if entity != 0 {
            held.calls.terminate_entity(entity);
        }
        held.calls
            .free_generator_slot(held.header.slots as usize + index * GENERATOR_SLOT_LEN);
    }
    say(format!(
        "enemies: generator {:#X}: a slot is taken out here ({}; set {}, {})",
        held.generator_id,
        match removal {
            SlotRemoval::NotTheHosts => "no spawn of the host stands behind it",
            SlotRemoval::OverOnTheHostBeforeItWasMade =>
                "its spawn is over on the host before the enemy was made here",
        },
        slot.set_index,
        if entity != 0 {
            format!("its enemy with handle {:#X} is removed", slot.handle)
        } else {
            "it held no enemy".to_owned()
        }
    ));
}

#[expect(
    unsafe_code,
    reason = "calls the add function of the generator's slot list"
)]
unsafe fn add_slot_for(held: &HeldUpdate, generator: usize, spawn: &EnemySpawn) {
    let own_object = held
        .calls
        .object_id_of_generator_set(&held.header, spawn.set_index);
    if own_object != Some(spawn.object_id) {
        with_guest_generators(|guest| guest.note_refused(spawn.id()));
        say(format!(
            "enemies: mismatch: the host made {}, but set {} of this copy's generator holds {}; it is not made here",
            spawn.label(),
            spawn.set_index,
            own_object.map_or("nothing".to_owned(), |object| format!("object {object:#X}"))
        ));
        return;
    }
    // SAFETY: forwarded from this function's own contract.
    let added = unsafe { held.calls.add_generator_slot(generator, spawn) };
    if added {
        with_guest_generators(|guest| guest.note_added(spawn.id()));
        say(format!(
            "enemies: {} gets a slot in this copy's generator",
            spawn.id().label()
        ));
    } else {
        with_guest_generators(|guest| guest.note_refused(spawn.id()));
        say(format!(
            "enemies: mismatch: this copy's generator took no slot for {}",
            spawn.label()
        ));
    }
}

fn note_slots_after_the_update(held: &HeldUpdate, generator: usize) {
    let Some(header) = held.calls.read_generator(generator) else {
        return;
    };
    let slots = held.calls.read_generator_slots(&header);
    let lines = with_guest_generators(|guest| guest.note_slots(held.generator_id, &slots));
    for line in lines.into_iter().flatten() {
        say(line);
    }
}

fn slot_before_its_step(slot: usize, generator: usize) -> Option<SlotBeforeItsStep> {
    let guard = WATCH.lock().ok()?;
    let watch = guard.as_ref()?;
    let calls = watch.calls;
    let generator_id = calls
        .read_u32_at((generator + GENERATOR_ACTION_ID_OFFSET) as u64)
        .ok()?;
    let slot = read_slot(&calls, slot)?;
    let generator_is_held = watch.guest_generators_are_held
        && !watch.generators_left_to_the_game.contains(&generator_id);
    let hosts_spawn = (generator_is_held && slot.waits_for_its_enemy())
        .then(|| watch.guest.spawn_in_slot(generator_id, &slot))
        .flatten();
    Some(SlotBeforeItsStep {
        calls,
        generator_id,
        slot,
        generator_is_held,
        hosts_spawn,
    })
}

fn note_a_spawn_of_this_copys_own(slot: usize, generator: usize, before: &SlotBeforeItsStep) {
    let Some(after) = read_slot(&before.calls, slot) else {
        return;
    };
    if before.slot.holds_a_made_enemy() || !after.holds_a_made_enemy() {
        return;
    }
    let Some(header) = before.calls.read_generator(generator) else {
        return;
    };
    let spawn_values = before
        .calls
        .spawn_values_of_generator_set(&header, after.set_index)
        .unwrap_or_default();
    let observed = ObservedSpawn {
        generator: before.generator_id,
        set_index: after.set_index,
        object_id: before
            .calls
            .object_id_of_generator_set(&header, after.set_index)
            .unwrap_or_default(),
        level: LEVEL_THE_GAME_PICKED_IN_THIS_STEP.get(),
        uses_the_second_set_values: header.uses_the_second_set_values,
        object_read_variant: spawn_values.0,
        set_values: spawn_values.1,
        handle: after.handle,
        position: after.position,
        rotation: after.rotation,
    };
    if let Ok(mut watch) = WATCH.lock()
        && let Some(watch) = watch.as_mut()
        && watch.observed.len() < MOST_OBSERVED_SPAWNS_KEPT_BETWEEN_TICKS
    {
        watch.observed.push(observed);
    }
}

fn read_slot(calls: &GameCalls, slot: usize) -> Option<GeneratorSlot> {
    let bytes = calls.read_bytes_at(slot as u64, GENERATOR_SLOT_LEN).ok()?;
    generator_slots(&bytes).into_iter().next()
}

fn say(line: String) {
    if let Ok(mut watch) = WATCH.lock()
        && let Some(watch) = watch.as_mut()
    {
        say_within(watch, line);
    }
}

fn say_within(watch: &mut SpawnWatch, line: String) {
    if watch.lines_left == 0 {
        return;
    }
    watch.lines_left -= 1;
    log_line(&line);
    if watch.lines_left == 0 {
        log_line("enemies: no more generator hook lines this session");
    }
}

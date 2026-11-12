use automata_game::character::Character;
use automata_game::partner::{
    BODY_PLAYER_CONTROLLED_OFFSET, ENTITY_HANDLE_OFFSET, SPAWNED_BODY_NAME,
};
use nc_hook::diagnostics::log_line;

use crate::game_calls::GameCalls;
use crate::game_hooks::{hold_body, release_body};

/// Spawns a body of the character, never linked into the hero's companion slot, and holds it in
/// the guards for the player's slot. The body drive's first correction teleports it to the
/// player. `None` when any step failed; what was spawned is removed again.
#[expect(unsafe_code, reason = "spawns or removes a body of this mod")]
pub fn make_body(
    calls: &GameCalls,
    slot: u8,
    character: Character,
    spawn_is_guarded: bool,
) -> Option<u32> {
    let object_id = character.object_id();
    if !spawn_is_guarded || !calls.object_data_is_loaded(object_id) {
        return None;
    }
    let mut name = SPAWNED_BODY_NAME.as_bytes().to_vec();
    name.push(0);
    // SAFETY: the data was just read as loaded and the wait guard is installed.
    let entity = unsafe { calls.spawn_by_name(&name, object_id) };
    if entity == 0 {
        log_line(&format!(
            "extra body of slot {slot}: the spawn of object {object_id:#X} returned 0"
        ));
        return None;
    }
    // SAFETY: the entity was returned by the game's spawn in this call and is alive.
    let body = unsafe { calls.as_player_body(entity) };
    let handle = calls
        .read_u32_at((entity + ENTITY_HANDLE_OFFSET as usize) as u64)
        .unwrap_or(0);
    if body == 0 || handle == 0 || !hold_body(handle, slot, false) {
        // SAFETY: the entity is the one this call spawned.
        unsafe { calls.terminate_entity(entity) };
        log_line(&format!(
            "extra body of slot {slot}: the spawned entity is no player body, has no handle or the guards hold no more bodies; removed it"
        ));
        return None;
    }
    log_line(&format!(
        "extra body of slot {slot}: spawned object {object_id:#X} as handle {handle:#X}, not linked into the companion slot"
    ));
    Some(handle)
}

/// Releases the body from the guards and terminates it while it is alive.
#[expect(unsafe_code, reason = "removes a body this mod spawned")]
pub fn remove_body(calls: &GameCalls, slot: u8, handle: u32) {
    release_body(handle);
    let body = calls.player_body_of_handle(handle);
    if body == 0 {
        return;
    }
    // Outside the guards a live body must not keep the player flag.
    let flag_cleared = calls
        .write_i32_at(calls.body_field(body, BODY_PLAYER_CONTROLLED_OFFSET), 0)
        .is_ok();
    if let Ok(entity) = calls.entity_of_body(body)
        && entity != 0
    {
        // SAFETY: the roster names only bodies this mod spawned, resolved from the live handle
        // just now.
        unsafe { calls.terminate_entity(entity) };
        log_line(&format!(
            "extra body of slot {slot}: removed handle {handle:#X} (player flag cleared first: {flag_cleared})"
        ));
    }
}

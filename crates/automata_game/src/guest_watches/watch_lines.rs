use crate::character::Character;
use crate::guest_watches::{WatchFeedRefusal, WatchedStretch};
use crate::load_gate::StoryPlace;

/// What the watch drive read this tick, for the line it writes once a second.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WatchingReading {
    pub hero_position: Option<[f32; 3]>,
    pub host_position: Option<[f32; 3]>,
    pub hero_state: Option<i32>,
    pub host_state: Option<u16>,
    pub hero_health: Option<i32>,
    pub host_health: Option<(i32, i32)>,
    pub hero_rides: bool,
    pub refusal: Option<WatchFeedRefusal>,
}

fn label_of(character: Option<Character>) -> &'static str {
    character.map_or("not read", Character::launcher_id)
}

pub fn watching_setting_line(on: bool, stretch: &WatchedStretch) -> String {
    if on {
        format!(
            "watching: guest_watches_the_opening is on: in the campaign the guest watches the host play {}; its own hero is driven as the host's body and neither copy has a partner body there",
            stretch.describe()
        )
    } else {
        "watching: guest_watches_the_opening is off, the guest plays its own body everywhere"
            .to_owned()
    }
}

pub fn watching_started_line(
    place: &StoryPlace,
    hero: Option<Character>,
    host: Option<Character>,
) -> String {
    format!(
        "watching: started at {}: this game's hero ({}) is driven as the host's body (the host plays {}); cut: the hero's pad, mouse and keyboard sources; fed: the host's held and pressed buttons, speed, facing and flight stick; its place and facing follow the host's, its health is the host's but never below 1 and no hit reaches it; no partner body is sought here, the pause menu and the skip prompt stay this player's",
        place.describe(),
        label_of(hero),
        label_of(host)
    )
}

pub fn watching_goes_on_outside_line(place: &StoryPlace) -> String {
    format!(
        "watching: this game is at {}, outside the watched stretch, without a full load; watching goes on until the next full load",
        place.describe()
    )
}

pub fn hand_back_line(target: &StoryPlace, own: Character, written: bool) -> String {
    format!(
        "watching: a full load to {} leaves the watched stretch; wrote this game's own character {} as the hero first (written {written}); watching ends when that load ends",
        target.describe(),
        own.launcher_id()
    )
}

pub fn watching_ended_by_the_load_line(place: &StoryPlace) -> String {
    format!(
        "watching: ended, the full load ended at {}: the hero's input sources are given back, a partner body is sought as always and the state message says not watching",
        place.describe()
    )
}

pub fn watching_ended_by_the_bound_line(place: &StoryPlace, waited_ms: u64) -> String {
    format!(
        "watching: ended, the full load asked for did not end within {waited_ms} ms (still at {}); the hero's input sources are given back anyway",
        place.describe()
    )
}

pub fn watching_ended_without_the_host_line(place: &StoryPlace) -> String {
    format!(
        "watching: ended at {}, this copy is no longer a campaign guest with its host present; the hero's input sources are given back",
        place.describe()
    )
}

pub fn start_hero_line(target: &StoryPlace, host: Character, written: bool) -> String {
    format!(
        "watching: the start goes to {}, which the guest watches; the hero is the host's character {} (written {written})",
        target.describe(),
        host.launcher_id()
    )
}

pub fn watching_line(reading: &WatchingReading) -> String {
    let error = reading
        .hero_position
        .zip(reading.host_position)
        .map(|(hero, host)| {
            let d = [host[0] - hero[0], host[1] - hero[1], host[2] - hero[2]];
            (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        });
    format!(
        "watching: the camera is on this game's hero, driven as the host's body; hero at {:?}, the host's body at {:?}, error {:?}; states {:X?} (hero) and {:X?} (host); health {:?} (hero), {:?} (host, of max){}; {}",
        reading.hero_position,
        reading.host_position,
        error,
        reading.hero_state,
        reading.host_state,
        reading.hero_health,
        reading.host_health,
        if reading.hero_rides {
            "; the hero rides a flight unit, its place is not corrected"
        } else {
            ""
        },
        match reading.refusal {
            None => "fed and placed".to_owned(),
            Some(refusal) => format!("held still: {refusal}"),
        }
    )
}

pub fn watch_refusal_line(refusal: WatchFeedRefusal) -> String {
    format!("watching: the hero is held still, nothing of the host is fed or corrected: {refusal}")
}

pub fn hero_was_held_line(handle: u32) -> String {
    format!(
        "watching: the hero (handle {handle:#X}) was in the held-body list of the guards; taken out, the guards never treat the hero as a held body"
    )
}

pub fn hero_teleported_line() -> String {
    "watching: first teleport of the hero to the host's place, with the player flag kept (the game's hero branch of the teleport runs; logged once)".to_owned()
}

pub fn host_watched_line(watched: bool) -> String {
    if watched {
        "watching: the guest watches this game's hero: no body is made or kept for the guest, the engine is not steered to make one, enemies aim at this game's hero only and the downed rule plays alone (a defeat is a defeat, nobody revives)".to_owned()
    } else {
        "watching: the guest no longer watches, its body is sought as always".to_owned()
    }
}

pub fn host_defeated_while_watched_line() -> String {
    "watching: this game's hero is defeated while the guest watches; the game's own defeat takes its way, the guest's hero is not defeated there and the guest comes to this game's place by the host-led stage".to_owned()
}

pub fn host_defeat_seen_while_watching_line(host_state: u16, host_health: i32) -> String {
    format!(
        "watching: the host is defeated (state {host_state:#X}, health {host_health}); this game's hero is not defeated here, this game comes to the host's place by the host-led stage (a move or the catch-up)"
    )
}

pub fn own_hits_kept_back_line(count: usize) -> String {
    format!(
        "watching: {count} hit(s) of this game's hero on enemies are not told to the host while watching (logged once)"
    )
}

pub fn own_pickups_kept_back_line(count: usize) -> String {
    format!(
        "watching: {count} item(s) given to this game's hero are not told to the host while watching (logged once)"
    )
}

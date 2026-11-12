use crate::enemies::EMPTY_ENTITY_HANDLES;
use crate::enemy_lead::{LeadTarget, TARGET_SWITCHES_WHEN_NEARER_BY};

/// Where the two players stand in the host's game: its own hero and the body the guest drives.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PlayerPlaces {
    pub host_player: Option<[f32; 3]>,
    pub guest_player: Option<[f32; 3]>,
}

/// The player a shared enemy should aim at on the host: the nearer one, kept until the other
/// is nearer by a clear margin. None leaves the target the game chose, which happens while the
/// enemy aims at neither player or the host's hero is not there.
pub fn target_wanted_on_the_host(
    enemy: [f32; 3],
    players: PlayerPlaces,
    current: LeadTarget,
) -> Option<LeadTarget> {
    if !matches!(current, LeadTarget::HostPlayer | LeadTarget::GuestPlayer) {
        return None;
    }
    let host = distance(enemy, players.host_player?);
    let Some(guest) = players.guest_player.map(|guest| distance(enemy, guest)) else {
        return Some(LeadTarget::HostPlayer);
    };
    let (kept, other, other_target) = match current {
        LeadTarget::GuestPlayer => (guest, host, LeadTarget::HostPlayer),
        _ => (host, guest, LeadTarget::GuestPlayer),
    };
    if other + TARGET_SWITCHES_WHEN_NEARER_BY < kept {
        Some(other_target)
    } else {
        Some(current)
    }
}

/// Names the handle an enemy aims at in words both copies share.
pub fn lead_target_of(
    target_handle: u32,
    host_player_handle: Option<u32>,
    guest_player_handle: Option<u32>,
) -> LeadTarget {
    if EMPTY_ENTITY_HANDLES.contains(&target_handle) {
        LeadTarget::Nobody
    } else if host_player_handle == Some(target_handle) {
        LeadTarget::HostPlayer
    } else if guest_player_handle == Some(target_handle) {
        LeadTarget::GuestPlayer
    } else {
        LeadTarget::SomethingElse
    }
}

/// The handle on the guest's copy for the player the host's enemy aims at: the host's player
/// is the partner body there, the guest's player is the guest's own hero.
pub fn guest_target_handle(
    target: LeadTarget,
    guest_hero_handle: Option<u32>,
    partner_body_handle: Option<u32>,
) -> Option<u32> {
    match target {
        LeadTarget::HostPlayer => partner_body_handle,
        LeadTarget::GuestPlayer => guest_hero_handle,
        LeadTarget::Nobody | LeadTarget::SomethingElse => None,
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let length = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if length.is_finite() { length } else { f32::MAX }
}

use crate::cube_probe::{ActorLink, LinkKind, NearbyActor};

pub fn grab_started_text(
    role: &str,
    subject: &str,
    motion_name: &str,
    position: [f32; 3],
    near_count: usize,
) -> String {
    format!(
        "cube probe: {role} {subject} grab started with motion {motion_name} at {}, {near_count} actors in range",
        point_text(position)
    )
}

pub fn nearby_actor_text(
    role: &str,
    subject: &str,
    module_base: u64,
    rank: usize,
    near: &NearbyActor,
) -> String {
    let vtable_rva = near.vtable.checked_sub(module_base).map_or_else(
        || "outside the exe".to_owned(),
        |rva| format!("rva {rva:#x}"),
    );
    format!(
        "cube probe: {role} {subject} near #{rank} actor {:#x} group {:#x} index {} class {} kind {:#x} serial {} \
         vtable {:#x} ({vtable_rva}) handle {:#x} at {} distance {:.2} texts [{}]",
        near.actor,
        near.group,
        near.index,
        near.class_id,
        near.kind_mask,
        near.spawn_serial,
        near.vtable,
        near.handle,
        point_text(near.position),
        near.distance,
        near.texts.join(", ")
    )
}

pub fn grab_link_text(
    role: &str,
    subject: &str,
    scanned_bytes: usize,
    links: &[ActorLink],
) -> String {
    if links.is_empty() {
        return format!(
            "cube probe: {role} {subject} grab link: none in the first {scanned_bytes:#x} bytes"
        );
    }
    let found: Vec<String> = links
        .iter()
        .map(|link| {
            let kind = match link.kind {
                LinkKind::Pointer => "pointer",
                LinkKind::Handle => "handle",
            };
            format!("+{:#x} {kind} to {:#x}", link.offset, link.actor)
        })
        .collect();
    format!(
        "cube probe: {role} {subject} grab link: {}",
        found.join(", ")
    )
}

pub fn held_position_text(
    role: &str,
    subject: &str,
    held: u64,
    chosen_by: &str,
    position: Option<[f32; 3]>,
    held_for_ms: u64,
) -> String {
    let position = position.map_or_else(|| "unreadable".to_owned(), point_text);
    format!(
        "cube probe: {role} {subject} holding {held:#x} ({chosen_by}) at {position} after {held_for_ms} ms"
    )
}

pub fn released_text(
    role: &str,
    subject: &str,
    held: Option<u64>,
    final_position: Option<[f32; 3]>,
    held_for_ms: u64,
    ended_by: &str,
) -> String {
    let held = held.map_or_else(|| "no actor".to_owned(), |actor| format!("{actor:#x}"));
    let position = final_position.map_or_else(|| "unreadable".to_owned(), point_text);
    format!(
        "cube probe: {role} {subject} {ended_by}: {held} final position {position} after {held_for_ms} ms"
    )
}

fn point_text(point: [f32; 3]) -> String {
    format!("({:.3} {:.3} {:.3})", point[0], point[1], point[2])
}

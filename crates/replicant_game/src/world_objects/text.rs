use replicant_proto::WorldObjectKey;

use crate::world_objects::{HoldHow, key_text};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HandleOwner {
    pub actor: u64,
    pub vtable_rva: u32,
    pub class_id: i32,
    pub position: Option<[f32; 3]>,
}

pub fn object_known_text(key: &WorldObjectKey, actor: u64, position: [f32; 3]) -> String {
    format!(
        "objects: {} is actor {actor:#x}, placed at {}",
        key_text(key),
        point_text(position)
    )
}

pub fn you_took_text(key: &WorldObjectKey, how: HoldHow, position: Option<[f32; 3]>) -> String {
    format!(
        "objects: you {} {} at {}",
        how_text(how),
        key_text(key),
        maybe_point_text(position)
    )
}

pub fn you_let_go_text(key: &WorldObjectKey, position: Option<[f32; 3]>) -> String {
    format!(
        "objects: you put down {} at {}",
        key_text(key),
        maybe_point_text(position)
    )
}

pub fn you_move_text(
    key: &WorldObjectKey,
    how: HoldHow,
    yours: Option<[f32; 3]>,
    partner_last_heard: Option<[f32; 3]>,
) -> String {
    format!(
        "objects: you {} {}: your copy at {}, the partner's copy last heard at {}",
        how_text(how),
        key_text(key),
        maybe_point_text(yours),
        maybe_point_text(partner_last_heard)
    )
}

pub fn partner_took_text(key: &WorldObjectKey, theirs: [f32; 3]) -> String {
    format!(
        "objects: the partner took {} at {}",
        key_text(key),
        point_text(theirs)
    )
}

pub fn partner_let_go_text(key: &WorldObjectKey, theirs: [f32; 3]) -> String {
    format!(
        "objects: the partner put down {} at {}",
        key_text(key),
        point_text(theirs)
    )
}

pub fn partner_moves_text(
    key: &WorldObjectKey,
    theirs: [f32; 3],
    yours: Option<[f32; 3]>,
) -> String {
    format!(
        "objects: the partner moves {}: their copy at {}, yours at {}",
        key_text(key),
        point_text(theirs),
        maybe_point_text(yours)
    )
}

pub fn refused_guest_move_text(key: &WorldObjectKey, yours: Option<[f32; 3]>) -> String {
    format!(
        "objects: the guest's move of {} refused, you hold it; your place {} sent back",
        key_text(key),
        maybe_point_text(yours)
    )
}

pub fn unlisted_hold_text(handle: u64, how: HoldHow, owner: Option<&HandleOwner>) -> String {
    let Some(owner) = owner else {
        return format!(
            "objects: you {} handle {handle:#x}, which the handle pool does not resolve",
            how_text(how)
        );
    };
    format!(
        "objects: you {} handle {handle:#x}: actor {:#x} vtable rva {:#x} class {:#x} at {}, not a listed kind",
        how_text(how),
        owner.actor,
        owner.vtable_rva,
        owner.class_id,
        maybe_point_text(owner.position)
    )
}

pub fn you_broke_text(key: &WorldObjectKey, i_am_host: bool) -> String {
    let route = if i_am_host {
        "sent to the guest"
    } else {
        "asked the host to break its copy"
    };
    format!("objects: you broke {} here, {route}", key_text(key))
}

pub fn partner_break_text(key: &WorldObjectKey, outcome: &str) -> String {
    format!("objects: the partner broke {}: {outcome}", key_text(key))
}

fn how_text(how: HoldHow) -> &'static str {
    match how {
        HoldHow::Push => "push",
        HoldHow::Carry => "carry",
    }
}

fn point_text(point: [f32; 3]) -> String {
    format!("({:.3} {:.3} {:.3})", point[0], point[1], point[2])
}

fn maybe_point_text(point: Option<[f32; 3]>) -> String {
    point.map_or_else(|| "unknown".to_owned(), point_text)
}

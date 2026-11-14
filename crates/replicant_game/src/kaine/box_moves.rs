use replicant_proto::PartnerBody;

pub const BODIES_WITHOUT_BOX_PUSH_OR_PULL_MOTIONS: [PartnerBody; 1] = [PartnerBody::SeedKaineE];

/// True when the local body was swapped to one whose motion set has no box push or pull
/// motions: the game's box state would hold it in its last pose, so the grab is refused.
pub fn swapped_body_cannot_move_boxes(body: PartnerBody, by_swap: bool) -> bool {
    by_swap && BODIES_WITHOUT_BOX_PUSH_OR_PULL_MOTIONS.contains(&body)
}

pub fn swapped_body_box_grab_refused_text(box_actor: u64) -> String {
    format!(
        "objects: refused your grab of box {box_actor:#x}: the swapped body has no box push or pull motions, the box stays the host's to move"
    )
}

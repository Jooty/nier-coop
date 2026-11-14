use replicant_proto::PartnerBody;

pub const BODIES_WITHOUT_BOX_PUSH_OR_PULL_MOTIONS: [PartnerBody; 1] = [PartnerBody::SeedKaineE];
pub const BOX_GRAB_COUNTS_AS_NEW_AFTER_A_GAP_OF_MS: u64 = 1_000;

/// Notes each new grab of a box by the swapped body once: the gate is asked every frame of a
/// grab, so a grab is new when the box changes or the gate went unasked for a while.
#[derive(Debug, Default)]
pub struct SwappedBoxGrabNote {
    last_box: u64,
    last_allowed_ms: u64,
}

impl SwappedBoxGrabNote {
    pub const fn new() -> Self {
        Self {
            last_box: 0,
            last_allowed_ms: 0,
        }
    }

    pub fn observe(&mut self, box_actor: u64, now_ms: u64) -> bool {
        let new_grab = box_actor != self.last_box
            || self.last_allowed_ms == 0
            || now_ms.saturating_sub(self.last_allowed_ms)
                >= BOX_GRAB_COUNTS_AS_NEW_AFTER_A_GAP_OF_MS;
        self.last_box = box_actor;
        self.last_allowed_ms = now_ms.max(1);
        new_grab
    }
}

pub fn swapped_body_moves_boxes_without_motions(body: PartnerBody, by_swap: bool) -> bool {
    by_swap && BODIES_WITHOUT_BOX_PUSH_OR_PULL_MOTIONS.contains(&body)
}

pub fn swapped_body_box_grab_text(box_actor: u64) -> String {
    format!(
        "objects: the swapped body is moving box {box_actor:#x} with no box motions (it has no box push or pull motions)"
    )
}

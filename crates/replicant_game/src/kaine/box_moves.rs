use replicant_proto::PartnerBody;

pub const BODIES_WITHOUT_BOX_PUSH_OR_PULL_MOTIONS: [PartnerBody; 1] = [PartnerBody::SeedKaineE];

/// Notes the swapped body entering the box move state once per box. It watches the state the game
/// runs, not the can-grab gate: the game asks that gate for every box in reach on every frame.
#[derive(Debug, Default)]
pub struct SwappedBoxMoveNote {
    box_in_running_state: Option<u64>,
}

impl SwappedBoxMoveNote {
    pub const fn new() -> Self {
        Self {
            box_in_running_state: None,
        }
    }

    pub fn observe(&mut self, box_move_handle: Option<u64>) -> bool {
        let entered = box_move_handle.is_some() && box_move_handle != self.box_in_running_state;
        self.box_in_running_state = box_move_handle;
        entered
    }
}

pub fn swapped_body_moves_boxes_without_motions(body: PartnerBody, by_swap: bool) -> bool {
    by_swap && BODIES_WITHOUT_BOX_PUSH_OR_PULL_MOTIONS.contains(&body)
}

pub fn swapped_body_box_move_text(box_handle: u64, box_actor: Option<u64>) -> String {
    let actor = box_actor.map_or_else(|| "unresolved".to_owned(), |actor| format!("{actor:#x}"));
    format!(
        "objects: the swapped body is moving box handle {box_handle:#x} (actor {actor}): it entered the box move state, with no box push or pull motions"
    )
}

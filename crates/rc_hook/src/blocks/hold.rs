use rc_game::GameMemory;
use rc_game::blocks::{BLOCK_GRAB_REACH, KnownBlock, distance_between};
use rc_game::cube_probe::{
    GrabWatch, LinkTarget, PLAYER_LINK_SCAN_BYTES, find_links, read_block_lossy,
};
use rc_game::offsets::{ACTOR_SELF_HANDLE_OFFSET, BODY_MOTION_LAYER};
use rc_game::puppet::{motion_name, read_layer_motion};

use crate::memory::InProcessMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct HeldBlock {
    pub id: u16,
    pub chosen_by: &'static str,
}

#[derive(Default)]
pub(super) struct LocalHold {
    watch: GrabWatch,
    actor: u64,
    cached_name: Option<(i32, Option<String>)>,
    held: Option<HeldBlock>,
}

impl LocalHold {
    pub(super) fn update(
        &mut self,
        memory: &InProcessMemory,
        now_ms: u64,
        actor: u64,
        position: Option<[f32; 3]>,
        blocks: &[(KnownBlock, Option<[f32; 3]>)],
    ) -> Option<HeldBlock> {
        if self.actor != actor {
            *self = Self {
                actor,
                ..Self::default()
            };
        }
        if actor == 0 {
            return None;
        }
        let name = self.body_motion_name(memory);
        self.watch.observe(now_ms, name.as_deref());
        if !self.watch.holding() {
            self.held = None;
            return None;
        }
        let linked = linked_block(memory, actor, blocks);
        if let Some(id) = linked {
            self.held = Some(HeldBlock {
                id,
                chosen_by: "grab link",
            });
        } else if self.held.is_none() {
            self.held = nearest_block(position, blocks).map(|id| HeldBlock {
                id,
                chosen_by: "nearest block, no link",
            });
        }
        self.held
    }

    fn body_motion_name(&mut self, memory: &InProcessMemory) -> Option<String> {
        let motion = read_layer_motion(memory, self.actor, BODY_MOTION_LAYER)
            .ok()
            .flatten()?;
        if let Some((index, name)) = &self.cached_name
            && *index == motion.index
        {
            return name.clone();
        }
        let name = motion_name(memory, self.actor, motion.index).ok().flatten();
        self.cached_name = Some((motion.index, name.clone()));
        name
    }
}

fn linked_block(
    memory: &InProcessMemory,
    player: u64,
    blocks: &[(KnownBlock, Option<[f32; 3]>)],
) -> Option<u16> {
    let targets: Vec<LinkTarget> = blocks
        .iter()
        .map(|(block, _)| LinkTarget {
            actor: block.actor,
            handle: memory
                .read_u64(block.actor + ACTOR_SELF_HANDLE_OFFSET)
                .unwrap_or(0),
        })
        .collect();
    let bytes = read_block_lossy(memory, player, PLAYER_LINK_SCAN_BYTES);
    let link = find_links(&bytes, &targets).into_iter().next()?;
    blocks
        .iter()
        .find(|(block, _)| block.actor == link.actor)
        .map(|(block, _)| block.id)
}

fn nearest_block(
    position: Option<[f32; 3]>,
    blocks: &[(KnownBlock, Option<[f32; 3]>)],
) -> Option<u16> {
    let player = position?;
    blocks
        .iter()
        .filter_map(|(block, at)| Some((block.id, distance_between(player, (*at)?))))
        .filter(|(_, distance)| *distance <= BLOCK_GRAB_REACH)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(id, _)| id)
}

use rc_proto::MAX_BLOCK_ID;

use crate::blocks::cell_of;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockSighting {
    pub actor: u64,
    pub position: [f32; 3],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KnownBlock {
    pub id: u16,
    pub actor: u64,
    pub start_cell: [i32; 3],
}

#[derive(Debug, Default)]
pub struct BlockRoster {
    phase: Option<u16>,
    blocks: Vec<KnownBlock>,
    next_id: u16,
}

impl BlockRoster {
    pub fn blocks(&self) -> &[KnownBlock] {
        &self.blocks
    }

    pub fn actor_of(&self, id: u16) -> Option<u64> {
        self.blocks
            .iter()
            .find(|block| block.id == id)
            .map(|block| block.actor)
    }

    pub fn id_of(&self, actor: u64) -> Option<u16> {
        self.blocks
            .iter()
            .find(|block| block.actor == actor)
            .map(|block| block.id)
    }

    pub fn observe(&mut self, phase: u16, sightings: &[BlockSighting]) -> Vec<KnownBlock> {
        if self.phase != Some(phase) {
            self.phase = Some(phase);
            self.forget_all();
        }
        let seen = |actor: u64| sightings.iter().any(|sighting| sighting.actor == actor);
        if !self.blocks.iter().any(|block| seen(block.actor)) {
            self.forget_all();
        }
        self.blocks.retain(|block| seen(block.actor));
        let mut fresh: Vec<&BlockSighting> = sightings
            .iter()
            .filter(|sighting| self.id_of(sighting.actor).is_none())
            .collect();
        fresh.sort_by_key(|sighting| (cell_of(sighting.position), sighting.actor));
        let mut added = Vec::new();
        for sighting in fresh {
            if self.next_id > MAX_BLOCK_ID {
                break;
            }
            let block = KnownBlock {
                id: self.next_id,
                actor: sighting.actor,
                start_cell: cell_of(sighting.position),
            };
            self.next_id += 1;
            self.blocks.push(block);
            added.push(block);
        }
        added
    }

    fn forget_all(&mut self) {
        self.blocks.clear();
        self.next_id = 0;
    }
}

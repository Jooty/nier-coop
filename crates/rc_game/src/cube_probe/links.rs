use crate::cube_probe::LINK_SCAN_CHUNK_BYTES;
use crate::memory::GameMemory;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkTarget {
    pub actor: u64,
    pub handle: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkKind {
    Pointer,
    Handle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActorLink {
    pub offset: u64,
    pub actor: u64,
    pub kind: LinkKind,
}

pub fn read_block_lossy<M: GameMemory>(memory: &M, address: u64, len: usize) -> Vec<u8> {
    let mut block = vec![0_u8; len];
    for (index, chunk) in block.chunks_mut(LINK_SCAN_CHUNK_BYTES).enumerate() {
        let start = address + (index * LINK_SCAN_CHUNK_BYTES) as u64;
        if memory.read(start, chunk).is_err() {
            chunk.fill(0);
        }
    }
    block
}

pub fn find_links(block: &[u8], targets: &[LinkTarget]) -> Vec<ActorLink> {
    let mut links = Vec::new();
    for offset in (0..block.len().saturating_sub(7)).step_by(4) {
        let value = u64::from_le_bytes([
            block[offset],
            block[offset + 1],
            block[offset + 2],
            block[offset + 3],
            block[offset + 4],
            block[offset + 5],
            block[offset + 6],
            block[offset + 7],
        ]);
        if value == 0 {
            continue;
        }
        for target in targets {
            let kind = if value == target.actor {
                LinkKind::Pointer
            } else if value == target.handle {
                LinkKind::Handle
            } else {
                continue;
            };
            links.push(ActorLink {
                offset: offset as u64,
                actor: target.actor,
                kind,
            });
        }
    }
    links
}

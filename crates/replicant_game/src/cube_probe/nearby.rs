use replicant_proto::ENEMY_GROUP_COUNT;

use crate::cube_probe::{
    ACTOR_TEXT_SCAN_BYTES, HIGHEST_PLAUSIBLE_POINTER, LOWEST_PLAUSIBLE_POINTER,
    MAX_NEARBY_ACTORS_LOGGED, POINTED_TEXT_READ_BYTES, SMALLEST_READABLE_TEXT_LEN,
    read_block_lossy,
};
use crate::enemies::read_group_counts;
use crate::memory::GameMemory;
use crate::offsets::{
    ACTOR_CLASS_ID_OFFSET, ACTOR_KIND_MASK_OFFSET, ACTOR_SELF_HANDLE_OFFSET,
    ACTOR_SPAWN_SERIAL_OFFSET, ACTOR_TRANSFORM_BLOCK_BYTES, ACTOR_TRANSFORM_BLOCK_OFFSET,
    MAX_ACTORS_SCANNED_PER_GROUP,
};
use crate::puppet::PuppetPose;

#[derive(Debug, Clone, PartialEq)]
pub struct NearbyActor {
    pub actor: u64,
    pub group: u16,
    pub index: i32,
    pub position: [f32; 3],
    pub distance: f32,
    pub class_id: i32,
    pub kind_mask: i32,
    pub spawn_serial: i32,
    pub vtable: u64,
    pub handle: u64,
    pub texts: Vec<String>,
}

pub fn distance_between_points(from: [f32; 3], to: [f32; 3]) -> f32 {
    (to[0] - from[0])
        .hypot(to[1] - from[1])
        .hypot(to[2] - from[2])
}

pub fn nearest_actor_index(actors: &[NearbyActor]) -> Option<usize> {
    actors
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| a.distance.total_cmp(&b.distance))
        .map(|(index, _)| index)
}

pub fn ascii_runs(bytes: &[u8], smallest: usize) -> Vec<(usize, String)> {
    let mut runs = Vec::new();
    let mut start = None;
    for (index, byte) in bytes.iter().copied().chain(core::iter::once(0)).enumerate() {
        let printable = (0x20..0x7F).contains(&byte);
        match (printable, start) {
            (true, None) => start = Some(index),
            (false, Some(from)) => {
                if index - from >= smallest {
                    let text = String::from_utf8_lossy(&bytes[from..index]).into_owned();
                    runs.push((from, text));
                }
                start = None;
            }
            _ => {}
        }
    }
    runs
}

pub fn read_nearby_actors<M: GameMemory>(
    memory: &M,
    center: [f32; 3],
    range: f32,
    skipped: &[u64],
    mut actor_in_group: impl FnMut(u16, i32) -> u64,
) -> Vec<NearbyActor> {
    let Ok(counts) = read_group_counts(memory) else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for (group, count) in (0..ENEMY_GROUP_COUNT).zip(counts) {
        for index in 0..count.clamp(0, MAX_ACTORS_SCANNED_PER_GROUP) {
            let actor = actor_in_group(group, index);
            if actor == 0 || skipped.contains(&actor) {
                continue;
            }
            if let Some(near) = read_if_near(memory, center, range, group, index, actor) {
                found.push(near);
            }
        }
    }
    found.sort_by(|a, b| a.distance.total_cmp(&b.distance));
    found.truncate(MAX_NEARBY_ACTORS_LOGGED);
    found
}

fn read_if_near<M: GameMemory>(
    memory: &M,
    center: [f32; 3],
    range: f32,
    group: u16,
    index: i32,
    actor: u64,
) -> Option<NearbyActor> {
    let block = memory
        .read_array::<ACTOR_TRANSFORM_BLOCK_BYTES>(actor + ACTOR_TRANSFORM_BLOCK_OFFSET)
        .ok()?;
    let pose = PuppetPose::from_transform_block(&block);
    let position = [pose.position.x, pose.position.y, pose.position.z];
    let distance = distance_between_points(center, position);
    if !distance.is_finite() || distance > range {
        return None;
    }
    Some(NearbyActor {
        actor,
        group,
        index,
        position,
        distance,
        class_id: memory.read_i32(actor + ACTOR_CLASS_ID_OFFSET).unwrap_or(-1),
        kind_mask: memory
            .read_i32(actor + ACTOR_KIND_MASK_OFFSET)
            .unwrap_or(-1),
        spawn_serial: memory
            .read_i32(actor + ACTOR_SPAWN_SERIAL_OFFSET)
            .unwrap_or(-1),
        vtable: memory.read_u64(actor).unwrap_or(0),
        handle: memory
            .read_u64(actor + ACTOR_SELF_HANDLE_OFFSET)
            .unwrap_or(0),
        texts: readable_texts(memory, actor),
    })
}

fn readable_texts<M: GameMemory>(memory: &M, actor: u64) -> Vec<String> {
    let block = read_block_lossy(memory, actor, ACTOR_TEXT_SCAN_BYTES);
    let mut texts: Vec<String> = ascii_runs(&block, SMALLEST_READABLE_TEXT_LEN)
        .into_iter()
        .map(|(offset, text)| format!("+{offset:#x} \"{text}\""))
        .collect();
    for offset in (0..block.len().saturating_sub(7)).step_by(8) {
        let pointer = u64::from_le_bytes(block[offset..offset + 8].try_into().unwrap_or([0; 8]));
        if !(LOWEST_PLAUSIBLE_POINTER..=HIGHEST_PLAUSIBLE_POINTER).contains(&pointer) {
            continue;
        }
        let Ok(bytes) = memory.read_array::<POINTED_TEXT_READ_BYTES>(pointer) else {
            continue;
        };
        if let Some((0, text)) = ascii_runs(&bytes, SMALLEST_READABLE_TEXT_LEN)
            .into_iter()
            .next()
        {
            texts.push(format!("+{offset:#x}->\"{text}\""));
        }
    }
    texts
}

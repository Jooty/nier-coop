use replicant_proto::StoryArray;

use crate::offsets::PHASE_NAME_BUFFER_LEN;
use crate::story_progress::arrays::fnv_hash;
use crate::story_progress::{
    PLAYER_NAME_BYTES, RANK_SIDECAR_MAGIC, RANK_SIDECAR_VERSION, SlotStory, StoryArrays,
};

const SAVE_POINT_SIDECAR_BYTES: usize = 4;
const SIDECAR_HEADER_BYTES: usize = 16;

pub fn encode_high_water_sidecar(story: &SlotStory, name: &[u8; PLAYER_NAME_BYTES]) -> Vec<u8> {
    let mut payload = Vec::new();
    payload.extend_from_slice(name);
    payload.extend_from_slice(&story.phase_name);
    payload.extend_from_slice(&story.save_point);
    for array in StoryArray::ALL {
        payload.extend_from_slice(story.arrays.bytes(array));
    }
    let mut bytes = Vec::with_capacity(SIDECAR_HEADER_BYTES + payload.len());
    bytes.extend_from_slice(&RANK_SIDECAR_MAGIC);
    bytes.push(RANK_SIDECAR_VERSION);
    bytes.extend_from_slice(&[0; 3]);
    bytes.extend_from_slice(&fnv_hash(&payload).to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes
}

pub fn decode_high_water_sidecar(bytes: &[u8]) -> Option<(SlotStory, [u8; PLAYER_NAME_BYTES])> {
    let payload_bytes = PLAYER_NAME_BYTES
        + PHASE_NAME_BUFFER_LEN
        + SAVE_POINT_SIDECAR_BYTES
        + StoryArray::ALL
            .iter()
            .map(|array| array.used_bytes())
            .sum::<usize>();
    if bytes.len() != SIDECAR_HEADER_BYTES + payload_bytes
        || bytes[..4] != RANK_SIDECAR_MAGIC
        || bytes[4] != RANK_SIDECAR_VERSION
    {
        return None;
    }
    let stored = u64::from_le_bytes(bytes[8..SIDECAR_HEADER_BYTES].try_into().ok()?);
    let payload = &bytes[SIDECAR_HEADER_BYTES..];
    if stored != fnv_hash(payload) {
        return None;
    }
    let (name, rest) = payload.split_at(PLAYER_NAME_BYTES);
    let (phase_name, rest) = rest.split_at(PHASE_NAME_BUFFER_LEN);
    let (save_point, mut rest) = rest.split_at(SAVE_POINT_SIDECAR_BYTES);
    let mut arrays = StoryArrays::default();
    for array in StoryArray::ALL {
        let (part, after) = rest.split_at(array.used_bytes());
        arrays.set_bytes(array, part);
        rest = after;
    }
    let story = SlotStory {
        arrays,
        phase_name: phase_name.try_into().ok()?,
        save_point: save_point.try_into().ok()?,
    };
    Some((story, name.try_into().ok()?))
}

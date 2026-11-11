use replicant_proto::{MUSIC_LAYER_COUNT, MusicState, NO_MUSIC_TRACK};

use crate::offsets::{
    AUDIO_MAN_LAYER_VOLUME_STRIDE, AUDIO_MAN_LAYER_VOLUMES_OFFSET, AUDIO_MAN_MASTER_VOLUME_OFFSET,
    AUDIO_MAN_RVA, AUDIO_MAN_STOP_FADE_SECONDS_OFFSET, AUDIO_MAN_STOP_RUNNING_OFFSET,
    BGM_CHANNEL_EVENT_TRACK_OFFSET, BGM_CHANNEL_GAME_OBJECT_POINTER_OFFSET, BGM_CHANNEL_OFFSET,
    BGM_CHANNEL_PLAYING_ID_OFFSET, BGM_CHANNEL_STATE_OFFSET, BGM_CHANNEL_TRACK_OFFSET,
    BGM_CHANNEL_TRACK_VOLUME_OFFSET, WWISE_GAME_OBJECT_ID_OFFSET,
};
use crate::{GameMemory, MemoryError};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MusicChannel {
    pub state: i32,
    pub track: i32,
    pub track_volume: f32,
    pub event_track: i32,
    pub playing_id: u32,
    pub game_object: u64,
    pub master_volume: f32,
    pub layer_volumes: [f32; MUSIC_LAYER_COUNT],
    pub stopping: bool,
    pub stop_fade_seconds: f32,
}

impl MusicChannel {
    pub fn to_wire(&self, position_ms: u32) -> MusicState {
        MusicState {
            track: self.track.max(NO_MUSIC_TRACK),
            position_ms,
            track_volume: self.track_volume,
            master_volume: self.master_volume,
            layer_volumes: self.layer_volumes,
            stopping: self.stopping,
            stop_fade_seconds: self.stop_fade_seconds,
        }
    }

    pub fn has_track(&self) -> bool {
        self.track >= 0
    }
}

pub fn read_music_channel<M: GameMemory>(memory: &M) -> Result<MusicChannel, MemoryError> {
    let audio_man = memory.module_base() + u64::from(AUDIO_MAN_RVA);
    let channel = audio_man + BGM_CHANNEL_OFFSET;
    let object_pointer = memory.read_u64(channel + BGM_CHANNEL_GAME_OBJECT_POINTER_OFFSET)?;
    let game_object = if object_pointer == 0 {
        0
    } else {
        memory.read_u64(object_pointer + WWISE_GAME_OBJECT_ID_OFFSET)?
    };
    let mut layer_volumes = [0.0; MUSIC_LAYER_COUNT];
    for (index, volume) in (0_u64..).zip(layer_volumes.iter_mut()) {
        let address =
            audio_man + AUDIO_MAN_LAYER_VOLUMES_OFFSET + index * AUDIO_MAN_LAYER_VOLUME_STRIDE;
        *volume = memory.read_f32(address)?;
    }
    Ok(MusicChannel {
        state: memory.read_i32(channel + BGM_CHANNEL_STATE_OFFSET)?,
        track: memory.read_i32(channel + BGM_CHANNEL_TRACK_OFFSET)?,
        track_volume: memory.read_f32(channel + BGM_CHANNEL_TRACK_VOLUME_OFFSET)?,
        event_track: memory.read_i32(channel + BGM_CHANNEL_EVENT_TRACK_OFFSET)?,
        playing_id: u32::from_le_bytes(memory.read_array(channel + BGM_CHANNEL_PLAYING_ID_OFFSET)?),
        game_object,
        master_volume: memory.read_f32(audio_man + AUDIO_MAN_MASTER_VOLUME_OFFSET)?,
        layer_volumes,
        stopping: memory.read_u8(audio_man + AUDIO_MAN_STOP_RUNNING_OFFSET)? != 0,
        stop_fade_seconds: memory.read_f32(audio_man + AUDIO_MAN_STOP_FADE_SECONDS_OFFSET)?,
    })
}

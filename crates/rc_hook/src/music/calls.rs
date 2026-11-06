use core::ffi::c_void;

use rc_game::GameMemory;
use rc_game::offsets::{
    AK_GET_EVENT_ID_FROM_PLAYING_ID_RVA, AK_GET_SOURCE_PLAY_POSITION_RVA, AK_RESULT_SUCCESS,
    AK_SEEK_ON_EVENT_RVA, AUDIO_MAN_RVA, MUSIC_INIT_PLAYING_ID_RVA, PLAY_BGM_FUNCTION_RVA,
    SET_BGM_LAYER_VOLUME_FUNCTION_RVA, SET_BGM_MASTER_VOLUME_FUNCTION_RVA, STOP_BGM_FUNCTION_RVA,
};

use crate::build_gate::VerifiedBuild;
use crate::memory::InProcessMemory;
use crate::music::hooks::applying;

type PlayBgm = unsafe extern "system" fn(*mut c_void, i32, f32, f32) -> u64;
type StopBgm = unsafe extern "system" fn(*mut c_void, f32) -> u64;
type SetMasterVolume = unsafe extern "system" fn(*mut c_void, f32, f32);
type SetLayerVolume = unsafe extern "system" fn(*mut c_void, i32, f32, f32);
type GetSourcePlayPosition = unsafe extern "system" fn(u32, *mut i32, bool) -> i32;
type SeekOnEvent = unsafe extern "system" fn(u32, u64, i32, bool, u32) -> i32;
type GetEventIdFromPlayingId = unsafe extern "system" fn(u32) -> u32;

fn audio_man(build: &VerifiedBuild) -> *mut c_void {
    (build.module_base() + u64::from(AUDIO_MAN_RVA)) as usize as *mut c_void
}

fn function(build: &VerifiedBuild, rva: u32) -> usize {
    (build.module_base() + u64::from(rva)) as usize
}

pub(super) fn play(build: &VerifiedBuild, track: i32, volume: f32, fade_seconds: f32) {
    let _applying = applying();
    #[expect(
        unsafe_code,
        reason = "calls the game's own PlayBgm at its verified address"
    )]
    // SAFETY: the RVA starts CAudioMan::PlayBgm(this, id, volume, fade seconds) in the verified
    // build, `this` is the audio manager singleton, and this runs on the game thread.
    unsafe {
        core::mem::transmute::<usize, PlayBgm>(function(build, PLAY_BGM_FUNCTION_RVA))(
            audio_man(build),
            track,
            volume,
            fade_seconds,
        );
    }
}

pub(super) fn stop(build: &VerifiedBuild, fade_seconds: f32) {
    let _applying = applying();
    #[expect(
        unsafe_code,
        reason = "calls the game's own StopBgm at its verified address"
    )]
    // SAFETY: the RVA starts CAudioMan::StopBgm(this, fade seconds) in the verified build.
    unsafe {
        core::mem::transmute::<usize, StopBgm>(function(build, STOP_BGM_FUNCTION_RVA))(
            audio_man(build),
            fade_seconds,
        );
    }
}

pub(super) fn set_master_volume(build: &VerifiedBuild, volume: f32, ramp_seconds: f32) {
    let _applying = applying();
    #[expect(
        unsafe_code,
        reason = "calls the game's own SetBgmMasterVolume at its verified address"
    )]
    // SAFETY: the RVA starts CAudioMan::SetBgmMasterVolume(this, volume, seconds) in the
    // verified build.
    unsafe {
        core::mem::transmute::<usize, SetMasterVolume>(function(
            build,
            SET_BGM_MASTER_VOLUME_FUNCTION_RVA,
        ))(audio_man(build), volume, ramp_seconds);
    }
}

pub(super) fn set_layer_volume(build: &VerifiedBuild, layer: i32, volume: f32, ramp_seconds: f32) {
    let _applying = applying();
    #[expect(
        unsafe_code,
        reason = "calls the game's own SetBgmLayerVolume at its verified address"
    )]
    // SAFETY: the RVA starts CAudioMan::SetBgmLayerVolume(this, layer 0..3, volume, seconds) in
    // the verified build.
    unsafe {
        core::mem::transmute::<usize, SetLayerVolume>(function(
            build,
            SET_BGM_LAYER_VOLUME_FUNCTION_RVA,
        ))(audio_man(build), layer, volume, ramp_seconds);
    }
}

pub(super) fn source_position_ms(build: &VerifiedBuild, playing_id: u32) -> Option<u32> {
    if playing_id == 0 {
        return None;
    }
    let mut position = 0_i32;
    #[expect(
        unsafe_code,
        reason = "calls the Wwise position query at its verified address"
    )]
    // SAFETY: the RVA starts AK::SoundEngine::GetSourcePlayPosition(playing id, out ms,
    // extrapolate) in the verified build; `position` outlives the call.
    let result = unsafe {
        core::mem::transmute::<usize, GetSourcePlayPosition>(function(
            build,
            AK_GET_SOURCE_PLAY_POSITION_RVA,
        ))(playing_id, &mut position, true)
    };
    (result == AK_RESULT_SUCCESS)
        .then(|| u32::try_from(position).ok())
        .flatten()
}

pub(super) fn music_init_position_ms(build: &VerifiedBuild) -> Option<u32> {
    let playing_id = InProcessMemory::new(build)
        .read_array(build.module_base() + u64::from(MUSIC_INIT_PLAYING_ID_RVA))
        .map(u32::from_le_bytes)
        .ok()?;
    source_position_ms(build, playing_id)
}

pub(super) fn seek(
    build: &VerifiedBuild,
    playing_id: u32,
    game_object: u64,
    position_ms: i32,
) -> Result<i32, &'static str> {
    #[expect(
        unsafe_code,
        reason = "calls the Wwise event-id lookup at its verified address"
    )]
    // SAFETY: the RVA starts AK::SoundEngine::Query::GetEventIDFromPlayingID(playing id) in the
    // verified build.
    let event_id = unsafe {
        core::mem::transmute::<usize, GetEventIdFromPlayingId>(function(
            build,
            AK_GET_EVENT_ID_FROM_PLAYING_ID_RVA,
        ))(playing_id)
    };
    if event_id == 0 {
        return Err("the playing id has no event");
    }
    #[expect(unsafe_code, reason = "calls the Wwise seek at its verified address")]
    // SAFETY: the RVA starts AK::SoundEngine::SeekOnEvent(event id, game object, ms, nearest
    // marker, playing id) in the verified build.
    let result = unsafe {
        core::mem::transmute::<usize, SeekOnEvent>(function(build, AK_SEEK_ON_EVENT_RVA))(
            event_id,
            game_object,
            position_ms,
            false,
            playing_id,
        )
    };
    Ok(result)
}

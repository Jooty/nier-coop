use rc_game::music::{
    GuestSync, LocalMusic, MUSIC_DRIFT_RESEEK_MS, MUSIC_MIN_CHANGE_INTERVAL_MS,
    MUSIC_REFRESH_INTERVAL_MS, MUSIC_SEEK_WAIT_LIMIT_MS, MUSIC_TARGET_STALE_MS, MusicChannel,
    MusicSender, MusicStep, read_music_channel,
};
use rc_game::offsets::{
    AUDIO_MAN_LAYER_VOLUMES_OFFSET, AUDIO_MAN_MASTER_VOLUME_OFFSET, AUDIO_MAN_RVA,
    AUDIO_MAN_STOP_FADE_SECONDS_OFFSET, AUDIO_MAN_STOP_RUNNING_OFFSET,
    BGM_CHANNEL_EVENT_TRACK_OFFSET, BGM_CHANNEL_GAME_OBJECT_POINTER_OFFSET, BGM_CHANNEL_OFFSET,
    BGM_CHANNEL_PLAYING_ID_OFFSET, BGM_CHANNEL_STATE_EVENT_POSTED, BGM_CHANNEL_STATE_OFFSET,
    BGM_CHANNEL_TRACK_OFFSET, BGM_CHANNEL_TRACK_VOLUME_OFFSET, WWISE_GAME_OBJECT_ID_OFFSET,
};
use rc_proto::{MusicState, NO_MUSIC_TRACK};

use crate::fake_memory::{FAKE_MODULE_BASE, FakeMemory};

const TRACK: i32 = 1234;
const OTHER_TRACK: i32 = 99;
const LATENCY_MS: u64 = 40;
const OBJECT_POINTER: u64 = 0x2_0000_0000;
const OBJECT_ID: u64 = 0xABCD;

fn state(track: i32, position_ms: u32) -> MusicState {
    MusicState {
        track,
        position_ms,
        ..MusicState::silent()
    }
}

fn channel(track: i32) -> MusicChannel {
    MusicChannel {
        state: BGM_CHANNEL_STATE_EVENT_POSTED,
        track,
        track_volume: 1.0,
        event_track: track,
        playing_id: 77,
        game_object: OBJECT_ID,
        master_volume: 1.0,
        layer_volumes: [1.0; 4],
        stopping: false,
        stop_fade_seconds: 0.0,
    }
}

fn local(channel: MusicChannel, position_ms: Option<u32>) -> LocalMusic {
    LocalMusic {
        channel,
        position_ms,
    }
}

fn idle_channel() -> MusicChannel {
    MusicChannel {
        state: 0,
        track: NO_MUSIC_TRACK,
        event_track: NO_MUSIC_TRACK,
        playing_id: 0,
        game_object: 0,
        ..channel(NO_MUSIC_TRACK)
    }
}

#[test]
fn the_channel_is_read_from_the_audio_manager() {
    let mut memory = FakeMemory::new();
    let audio_man = FAKE_MODULE_BASE + u64::from(AUDIO_MAN_RVA);
    let channel_base = audio_man + BGM_CHANNEL_OFFSET;
    memory.put(
        channel_base + BGM_CHANNEL_STATE_OFFSET,
        &3_i32.to_le_bytes(),
    );
    memory.put(
        channel_base + BGM_CHANNEL_TRACK_OFFSET,
        &TRACK.to_le_bytes(),
    );
    memory.put(
        channel_base + BGM_CHANNEL_TRACK_VOLUME_OFFSET,
        &0.5_f32.to_le_bytes(),
    );
    memory.put(
        channel_base + BGM_CHANNEL_EVENT_TRACK_OFFSET,
        &TRACK.to_le_bytes(),
    );
    memory.put(
        channel_base + BGM_CHANNEL_PLAYING_ID_OFFSET,
        &77_u32.to_le_bytes(),
    );
    memory.put(
        channel_base + BGM_CHANNEL_GAME_OBJECT_POINTER_OFFSET,
        &OBJECT_POINTER.to_le_bytes(),
    );
    memory.put(
        OBJECT_POINTER + WWISE_GAME_OBJECT_ID_OFFSET,
        &OBJECT_ID.to_le_bytes(),
    );
    memory.put(
        audio_man + AUDIO_MAN_MASTER_VOLUME_OFFSET,
        &0.8_f32.to_le_bytes(),
    );
    memory.put(audio_man + AUDIO_MAN_STOP_RUNNING_OFFSET, &[1]);
    memory.put(
        audio_man + AUDIO_MAN_STOP_FADE_SECONDS_OFFSET,
        &2.0_f32.to_le_bytes(),
    );
    for layer in 0..4_u64 {
        memory.put(
            audio_man + AUDIO_MAN_LAYER_VOLUMES_OFFSET + layer * 4,
            &(layer as f32 / 4.0).to_le_bytes(),
        );
    }
    let read = read_music_channel(&memory).expect("channel is readable");
    assert_eq!(read.track, TRACK);
    assert_eq!(read.playing_id, 77);
    assert_eq!(read.game_object, OBJECT_ID);
    assert_eq!(read.layer_volumes, [0.0, 0.25, 0.5, 0.75]);
    assert!(read.stopping);
    let wire = read.to_wire(5000);
    assert_eq!(wire.position_ms, 5000);
    assert!((wire.master_volume - 0.8).abs() < 1e-6);
    assert!((wire.stop_fade_seconds - 2.0).abs() < 1e-6);
    assert_eq!(wire.validate(), Ok(()));
}

#[test]
fn the_sender_sends_first_then_on_change_then_about_once_a_second() {
    let mut sender = MusicSender::new();
    let mut now = 0;
    assert!(sender.should_send(now, &state(TRACK, 0)));
    now += 500;
    assert!(!sender.should_send(now, &state(TRACK, 500)));
    now += MUSIC_REFRESH_INTERVAL_MS;
    assert!(sender.should_send(now, &state(TRACK, 1500)));
    now += MUSIC_MIN_CHANGE_INTERVAL_MS;
    assert!(sender.should_send(now, &state(OTHER_TRACK, 0)));
}

#[test]
fn a_change_is_held_back_a_moment_so_a_burst_cannot_flood() {
    let mut sender = MusicSender::new();
    assert!(sender.should_send(0, &state(TRACK, 0)));
    assert!(!sender.should_send(MUSIC_MIN_CHANGE_INTERVAL_MS - 1, &state(OTHER_TRACK, 0)));
    assert!(sender.should_send(MUSIC_MIN_CHANGE_INTERVAL_MS, &state(OTHER_TRACK, 0)));
}

#[test]
fn a_new_partner_gets_the_state_at_once() {
    let mut sender = MusicSender::new();
    assert!(sender.should_send(0, &state(TRACK, 0)));
    sender.send_next_now();
    assert!(sender.should_send(10, &state(TRACK, 10)));
}

#[test]
fn a_volume_change_is_a_change() {
    let mut sender = MusicSender::new();
    assert!(sender.should_send(0, &state(TRACK, 0)));
    let mut quieter = state(TRACK, 200);
    quieter.layer_volumes[2] = 0.0;
    assert!(sender.should_send(MUSIC_MIN_CHANGE_INTERVAL_MS, &quieter));
}

#[test]
fn the_guest_starts_the_hosts_track_then_seeks_once_it_is_playing() {
    let mut sync = GuestSync::new();
    let mut host = state(TRACK, 10_000);
    host.track_volume = 0.9;
    sync.receive(host, 1000);
    let steps = sync.plan(&local(idle_channel(), None), 1000, LATENCY_MS);
    assert_eq!(
        steps,
        vec![MusicStep::Play {
            track: TRACK,
            volume: 0.9
        }]
    );
    let requested = MusicChannel {
        state: 2,
        playing_id: 0,
        game_object: 0,
        event_track: NO_MUSIC_TRACK,
        ..channel(TRACK)
    };
    assert!(
        sync.plan(&local(requested, None), 1100, LATENCY_MS)
            .is_empty()
    );
    let steps = sync.plan(&local(channel(TRACK), Some(0)), 1300, LATENCY_MS);
    assert_eq!(
        steps,
        vec![MusicStep::Seek {
            position_ms: 10_000 + 300 + 40
        }]
    );
    assert!(
        sync.plan(&local(channel(TRACK), Some(340)), 1400, LATENCY_MS)
            .is_empty()
    );
}

#[test]
fn the_guest_already_on_the_track_only_seeks_when_it_has_drifted() {
    let mut sync = GuestSync::new();
    sync.receive(state(TRACK, 20_000), 0);
    let close = 20_000 + LATENCY_MS as u32 + MUSIC_DRIFT_RESEEK_MS;
    assert!(
        sync.plan(&local(channel(TRACK), Some(close)), 0, LATENCY_MS)
            .is_empty()
    );
    let far = 20_000 + LATENCY_MS as u32 + MUSIC_DRIFT_RESEEK_MS + 1;
    let steps = sync.plan(&local(channel(TRACK), Some(far)), 0, LATENCY_MS);
    assert!(matches!(steps.as_slice(), [MusicStep::Seek { .. }]));
}

#[test]
fn drift_seeks_are_spaced_out() {
    let mut sync = GuestSync::new();
    sync.receive(state(TRACK, 0), 0);
    let off = local(channel(TRACK), Some(60_000));
    assert_eq!(sync.plan(&off, 0, 0).len(), 1);
    assert!(sync.plan(&off, 1000, 0).is_empty());
    assert_eq!(sync.plan(&off, 6000, 0).len(), 1);
}

#[test]
fn the_expected_position_moves_with_time_and_latency() {
    let mut sync = GuestSync::new();
    sync.receive(state(TRACK, 1000), 500);
    assert_eq!(sync.expected_position_ms(2500, 40), Some(3040));
    let mut stopping = state(TRACK, 1000);
    stopping.stopping = true;
    sync.receive(stopping, 500);
    assert_eq!(sync.expected_position_ms(2500, 40), Some(1000));
}

#[test]
fn a_track_that_never_gets_its_event_is_given_up_on() {
    let mut sync = GuestSync::new();
    sync.receive(state(TRACK, 0), 0);
    sync.plan(&local(idle_channel(), None), 0, 0);
    let switching = MusicChannel {
        event_track: OTHER_TRACK,
        ..channel(TRACK)
    };
    assert!(sync.plan(&local(switching, None), 1000, 0).is_empty());
    let steps = sync.plan(&local(switching, None), MUSIC_SEEK_WAIT_LIMIT_MS, 0);
    assert_eq!(
        steps,
        vec![MusicStep::SeekGaveUp {
            event_track: OTHER_TRACK,
            state: BGM_CHANNEL_STATE_EVENT_POSTED
        }]
    );
    assert!(
        sync.plan(&local(switching, None), MUSIC_SEEK_WAIT_LIMIT_MS + 1, 0)
            .is_empty()
    );
}

#[test]
fn the_guest_puts_back_a_track_its_own_scripts_changed() {
    let mut sync = GuestSync::new();
    sync.receive(state(TRACK, 0), 0);
    let steps = sync.plan(&local(channel(OTHER_TRACK), Some(0)), 0, 0);
    assert!(matches!(
        steps.as_slice(),
        [MusicStep::Play { track: TRACK, .. }]
    ));
}

#[test]
fn the_guest_copies_master_and_layer_volumes() {
    let mut sync = GuestSync::new();
    let mut host = state(TRACK, 0);
    host.master_volume = 0.5;
    host.layer_volumes = [1.0, 0.0, 1.0, 0.25];
    sync.receive(host, 0);
    let steps = sync.plan(&local(channel(TRACK), Some(0)), 0, 0);
    assert_eq!(
        steps,
        vec![
            MusicStep::Master { volume: 0.5 },
            MusicStep::Layer {
                layer: 1,
                volume: 0.0
            },
            MusicStep::Layer {
                layer: 3,
                volume: 0.25
            },
        ]
    );
}

#[test]
fn the_guest_stops_when_the_host_has_no_music_or_is_fading_out() {
    let mut sync = GuestSync::new();
    sync.receive(MusicState::silent(), 0);
    assert_eq!(
        sync.plan(&local(channel(TRACK), None), 0, 0),
        vec![MusicStep::Stop { fade_seconds: 0.0 }]
    );
    assert!(sync.plan(&local(idle_channel(), None), 1000, 0).is_empty());
    let mut fading = state(TRACK, 5000);
    fading.stopping = true;
    fading.stop_fade_seconds = 3.0;
    sync.receive(fading, 2000);
    assert_eq!(
        sync.plan(&local(channel(TRACK), None), 2000, 0),
        vec![MusicStep::Stop { fade_seconds: 3.0 }]
    );
    let already = MusicChannel {
        stopping: true,
        ..channel(TRACK)
    };
    assert!(sync.plan(&local(already, None), 3000, 0).is_empty());
}

#[test]
fn a_silent_host_state_goes_stale_and_hands_the_music_back() {
    let mut sync = GuestSync::new();
    assert!(!sync.is_fresh(0));
    sync.receive(state(TRACK, 0), 100);
    assert!(sync.is_fresh(100 + MUSIC_TARGET_STALE_MS));
    assert!(!sync.is_fresh(101 + MUSIC_TARGET_STALE_MS));
    sync.forget();
    assert!(sync.plan(&local(channel(TRACK), None), 0, 0).is_empty());
}

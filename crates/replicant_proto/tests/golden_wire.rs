use nt_wire::{BuildGate, MemberPing, STORY_MODE, SessionRules};
use replicant_proto::*;

const GOLDEN_HEX: &str = include_str!("golden_wire.hex");
const SKY_ASSET_NAME_COUNT: usize = 5;

fn pos() -> [f32; 3] {
    [1.5, -2.0, 3.25]
}

fn enemy_id() -> EnemyId {
    EnemyId {
        group: 3,
        ordinal: 4,
    }
}

fn motion() -> MotionState {
    MotionState {
        index: 7,
        frame: 2.5,
        speed: 1.0,
        looping: true,
        name_hash: 0xABCD_1234,
    }
}

fn enemy_state() -> EnemyState {
    EnemyState {
        id: enemy_id(),
        position: pos(),
        yaw_radians: 0.5,
        motion: Some(motion()),
        hp: 90,
        max_hp: 100,
    }
}

fn npc_state() -> NpcState {
    NpcState {
        id: NpcId {
            group: 1,
            class: 2,
            ordinal: 3,
        },
        position: pos(),
        yaw_radians: 0.25,
        motion: Some(motion()),
        visible: true,
    }
}

fn effect_play() -> EffectPlay {
    EffectPlay {
        effect: "fx".to_owned(),
        params: vec![1; EFFECT_PARAM_BYTES],
        attach: vec![3; EFFECT_ATTACH_BYTES],
        bone: "bone".to_owned(),
        second_bone: "bone2".to_owned(),
        handle: 9,
    }
}

fn story_call() -> StoryCall {
    StoryCall {
        phase: 5,
        pack: 0x1122_3344_5566,
        function: "fn_a".to_owned(),
        args: vec![
            StoryCallArg::Number(7),
            StoryCallArg::Int(-3),
            StoryCallArg::Text(vec![65, 66]),
        ],
    }
}

fn bullet_flight() -> BulletFlight {
    BulletFlight {
        instance: 12,
        transform: [0.5; BULLET_TRANSFORM_FLOATS],
        velocity: pos(),
        look: Some(BulletLook {
            effect_ids: [1, 2, 3, 4],
            styles: [[0.25; BULLET_STYLE_FLOATS]; BULLET_EFFECT_SLOTS],
        }),
        ended: false,
    }
}

fn callback_fired() -> CallbackFired {
    CallbackFired {
        phase: 6,
        name: "cb".to_owned(),
    }
}

fn gauge_open() -> AttackGaugeOpen {
    AttackGaugeOpen {
        boss: enemy_id(),
        partner: Some(EnemyId {
            group: 5,
            ordinal: 6,
        }),
        total_seconds: 30.0,
        remaining_seconds: 12.5,
        map_points: [1, 2, 3],
        options: [4, 5],
        nodes: vec![AttackGaugeNode {
            bone: 8,
            size: 1.5,
            offset: [0.5, 1.5, 2.5, 3.5],
            kind: 2,
            extra: 3,
            hp: 40,
            max_hp: 50,
        }],
    }
}

fn save_summary() -> SaveSummary {
    SaveSummary {
        name: "Save".to_owned(),
        level: 12,
        play_seconds: 3600.5,
    }
}

fn world_object_key() -> WorldObjectKey {
    WorldObjectKey {
        kind_vtable_rva: 0x1234_5678,
        group: 9,
        start_cell: [1, -2, 3],
    }
}

fn fight_args() -> Vec<StoryCallArg> {
    vec![StoryCallArg::Int(2), StoryCallArg::Text(vec![1])]
}

fn story_changes() -> Vec<StoryChange> {
    vec![
        StoryChange::Flag {
            array: StoryArray::GameFlags,
            id: 300,
            on: true,
        },
        StoryChange::Counter {
            array: StoryArray::SnowCounters,
            id: 4,
            value: 5,
        },
    ]
}

fn player_state() -> PlayerState {
    PlayerState {
        tick: 1000,
        phase: 2,
        position: pos(),
        yaw_radians: 1.0,
        motion: Some(motion()),
        hp: 80,
        max_hp: 100,
        weapon: Some(WeaponState {
            id: 3,
            level: 3,
            in_hand: true,
            visible: false,
        }),
        activity: PartnerActivity::InMenu,
        mp: 50,
        max_mp: 60,
        level: 9,
        ridden_boar: Some(RiddenBoar {
            offset_from_rider: pos(),
            yaw_from_rider_radians: 0.75,
            motion: Some(motion()),
        }),
    }
}

fn door_change() -> DoorChange {
    DoorChange {
        layout_id: 21,
        action: DoorAction::Open,
        flag: true,
        phase: 3,
    }
}

fn enemy_snapshot() -> EnemySnapshot {
    EnemySnapshot {
        tick: 77,
        phase: 2,
        part: 0,
        parts: 1,
        total: 1,
        round: 1,
        enemies: vec![enemy_state()],
    }
}

fn fight_camera_pose() -> FightCameraPose {
    FightCameraPose {
        eye: pos(),
        look_at: [4.0, 5.0, 6.0],
    }
}

fn session_rules() -> SessionRules {
    SessionRules {
        mode: STORY_MODE,
        max_players: 2,
        map: 3,
    }
}

fn samples() -> Vec<Message> {
    vec![
        Message::Hello(Hello {
            version: PROTOCOL_VERSION,
            build: BuildGate {
                time_date_stamp: 0x1122_3344,
                size_of_image: 0x0055_6677,
            },
            name: "Nier".to_owned(),
        }),
        Message::Welcome(Welcome {
            version: PROTOCOL_VERSION,
            slot: 1,
            host_name: "Host".to_owned(),
            rules: session_rules(),
        }),
        Message::Reject(Reject {
            reason: RejectReason::BuildMismatch,
        }),
        Message::State(player_state()),
        Message::Ping(Ping { nonce: 0xDEAD_BEEF }),
        Message::Pong(Pong { nonce: 0xFEED_FACE }),
        Message::Bye,
        Message::PathInfo(PathInfo {
            rtt_micros: 4200,
            direct: true,
        }),
        Message::Combat(CombatEvent::ReviveProgress(60)),
        Message::FollowPhase(FollowPhase {
            phase: 4,
            point: 5,
            in_side_view: true,
            last_normal_view: Some(pos()),
            side_view_door: 8,
        }),
        Message::EnemySnapshot(enemy_snapshot()),
        Message::EnemySpawned(EnemySpawned {
            phase: 2,
            enemy: enemy_state(),
        }),
        Message::EnemyGone(EnemyGone {
            phase: 2,
            id: enemy_id(),
            reason: EnemyGoneReason::Removed,
        }),
        Message::SessionOptions(SessionOptions {
            show_names: true,
            show_party_frame: false,
            show_distance: true,
            show_ping: false,
            enemy_health_percent: 150,
            enemy_damage_percent: 75,
        }),
        Message::Readiness(GameReadiness::InPlay),
        Message::Notice(PartnerNotice::HostWantsToGo { phase: Some(3) }),
        Message::Effect(effect_play()),
        Message::Sound(SoundPlay {
            sound: "snd".to_owned(),
        }),
        Message::EffectStop(EffectStop { handle: 9 }),
        Message::Settings(PlayerSettings {
            keep_running: true,
            event_log: false,
        }),
        Message::Music(MusicState {
            track: 14,
            position_ms: 5000,
            track_volume: 0.5,
            master_volume: 1.0,
            layer_volumes: [0.1, 0.2, 0.3, 0.4],
            stopping: true,
            stop_fade_seconds: 2.5,
        }),
        Message::Pickup(ItemPickup {
            item_id: 100,
            count: 3,
            source: PickupSource::Shop,
        }),
        Message::Sky(SkyState {
            phase: 2,
            env_area: 5,
            env_blend_seconds: 1.5,
            wind_area: 6,
            weather: 1,
            weather_offset: 2,
        }),
        Message::Dialogue(DialogueUpdate::StoryLine(DialogueLine {
            talk_id: 40,
            line: 3,
        })),
        Message::NpcSnapshot(NpcSnapshot {
            tick: 9,
            phase: 2,
            part: 0,
            parts: 1,
            total: 1,
            npcs: vec![npc_state()],
        }),
        Message::SkyRecord(SkyRecordChunk {
            phase: 2,
            area: 5,
            offset: 16,
            bytes: vec![9, 8, 7],
        }),
        Message::NpcTalk(NpcTalk {
            phase: 2,
            npc: npc_state(),
            talking: true,
        }),
        Message::SkyAssets(SkyAssets {
            phase: 2,
            area: 5,
            names: vec!["a".to_owned(); SKY_ASSET_NAME_COUNT],
        }),
        Message::Fade(FadeSync {
            fading_out: true,
            kind: 1,
            depth: 2,
            seconds: 0.75,
        }),
        Message::EnemyHit(EnemyHit {
            phase: 2,
            id: enemy_id(),
            hit_id: 11,
            hp_damage: 12,
            second_gauge: 13,
            third_gauge: 14,
            flags: 15,
            attack_kind: 16,
        }),
        Message::PlayerHit(PlayerHit {
            phase: 2,
            attacker: enemy_id(),
            hp_damage: 12,
            flags: 3,
            attack_kind: 4,
            hit_id: 5,
            push: 1.5,
            direction: [0.0, 1.0, 0.0],
        }),
        Message::EnemyKilled(EnemyKilled {
            phase: 2,
            id: enemy_id(),
            by_guest: true,
        }),
        Message::EnemyDrop(EnemyDrop {
            phase: 2,
            id: enemy_id(),
            item_id: 30,
            count: 2,
            position: pos(),
        }),
        Message::DoorChange(door_change()),
        Message::DoorRequest(door_change()),
        Message::DoorReport(DoorReport {
            phase: 3,
            doors: vec![DoorState {
                layout_id: 21,
                open: true,
                locked: false,
            }],
        }),
        Message::CutsceneSkip(CutsceneSkipAsk),
        Message::StoryRequest(StoryRequest),
        Message::StoryPart(StoryArrayPart {
            array: StoryArray::KeyFragments,
            bytes: vec![1; StoryArray::KeyFragments.used_bytes()],
        }),
        Message::StoryChanges(StoryChanges {
            changes: story_changes(),
        }),
        Message::StoryDigest(StoryDigest {
            hash: 0x0102_0304_0506_0708,
        }),
        Message::EnemyWordDrop(EnemyWordDrop {
            phase: 2,
            id: enemy_id(),
            word_id: 17,
        }),
        Message::ContainerOpened(ContainerOpened {
            phase: 2,
            action_id: 33,
            items: vec![ContainerItem {
                item_id: 5,
                count: 2,
            }],
            gold: 250,
            flags: vec![ContainerFlag {
                array: StoryArray::GameFlags,
                id: 12,
            }],
        }),
        Message::StoryWrite(StoryWrite {
            source: StoryWriteSource::LocalScene,
            changes: story_changes(),
        }),
        Message::TalkChoice(TalkChoice {
            talk_id: 41,
            line: 2,
            kind: TalkChoiceKind::MenuList,
            index: 1,
            target: 99,
        }),
        Message::StoryTrigger(story_call()),
        Message::StoryRun(story_call()),
        Message::ScriptRoll(ScriptRoll {
            phase: 2,
            call: 0x0A0B_0C0D_0E0F,
            ordinal: 3,
            value: ScriptRollValue::Float(0.5),
        }),
        Message::StoryRefused(StoryRefused {
            call: story_call(),
            reason: StoryRefusalReason::Duplicate,
        }),
        Message::ScriptMessage(ScriptMessage {
            phase: 2,
            message: 0x77,
            seq: 8,
        }),
        Message::EndingRecords(EndingRecords {
            clear_bits: 0b1011,
            costumes: [1, 2, 3, 4, 5, 6, 7],
            accomplished: vec![9; ENDING_ACCOMPLISHED_BYTES],
        }),
        Message::LobbyReady(LobbyReady {
            ready: true,
            mode: STORY_MODE,
            slot: Some(1),
        }),
        Message::LobbyStart(LobbyStart { mode: STORY_MODE }),
        Message::PartnerBody(BodyPackName::new("body_a").expect("valid name")),
        Message::TutorialPrompt(TutorialPrompt {
            id: FIRST_TUTORIAL_ID + 1,
        }),
        Message::BookEffect(effect_play()),
        Message::BookSound(SoundPlay {
            sound: "book".to_owned(),
        }),
        Message::BulletEffect(BulletEffectPlay {
            position: pos(),
            effect: effect_play(),
        }),
        Message::PrologueLevel(PrologueLevel {
            level: 3,
            experience: 450,
            weapon: 2,
            weapon_level: 1,
        }),
        Message::PowerModel(PowerModelState {
            kind: PowerModelKind::Bomb010,
            instance: 4,
            position: pos(),
            yaw_radians: 0.5,
        }),
        Message::BulletFlight(bullet_flight()),
        Message::SceneFollow(SceneFollow {
            phase: 2,
            callbacks: vec!["one".to_owned(), "two".to_owned()],
            cut: Some(SceneCut {
                event: "ev".to_owned(),
                audio: "au".to_owned(),
            }),
        }),
        Message::CallbackFired(callback_fired()),
        Message::Desync(DesyncDigest {
            phase: 2,
            position: pos(),
            story: Some(0x99AA_BBCC),
            groups: vec![GroupRoster {
                group: 3,
                count: 4,
                ids_hash: 0x1357_9BDF,
            }],
            groups_not_listed: 2,
        }),
        Message::EnemyBulletFlight(EnemyBulletFlight {
            enemy: enemy_id(),
            flight: bullet_flight(),
            sprite: Some(BulletSprite {
                kind: 1,
                size: 2.5,
                alpha: 0.5,
                angle: 1.5,
            }),
            scale_growth_per_second: 0.75,
        }),
        Message::PausedHits(PausedHits {
            paused_players_can_be_hit: true,
        }),
        Message::EnemyBodyGone(EnemyBodyGone {
            phase: 2,
            id: enemy_id(),
        }),
        Message::PowerMotion(PowerModelMotion {
            instance: 4,
            motion: motion(),
        }),
        Message::PowerEnded(PowerModelEnded {
            instance: 4,
            lasted_ms: 1500,
        }),
        Message::WorldEffect(WorldEffectPlay {
            effect: "world".to_owned(),
            transform: [0.5; WORLD_EFFECT_TRANSFORM_FLOATS],
            arguments: [1, 2, 3, 4, 5],
            handle: 31,
            look: Some(EffectLook {
                style: [0.25; BULLET_STYLE_FLOATS],
                extra: 1.5,
                flag: 1,
            }),
        }),
        Message::WorldEffectMove(WorldEffectMove {
            handle: 31,
            transform: [0.75; WORLD_EFFECT_TRANSFORM_FLOATS],
        }),
        Message::VoiceCharacter(VoiceCharacter {
            state: "voice".to_owned(),
        }),
        Message::StoryJump(StoryJump {
            point: StoryJumpPoint::AtPopolaTalk,
        }),
        Message::LobbyBody(LobbyBody {
            body: PartnerBody::SeedNierF,
        }),
        Message::QuitGame,
        Message::ItemCounts(ItemCounts {
            replaces_all: true,
            entries: vec![ItemCountEntry {
                item_id: 7,
                count: 9,
            }],
        }),
        Message::ItemTake(ItemTake {
            item_id: 7,
            count: 2,
        }),
        Message::TeleportHere(TeleportHere {
            phase: 2,
            position: pos(),
        }),
        Message::EnemyCreation(EnemyCreation {
            phase: 2,
            id: enemy_id(),
            actor_name: "actor".to_owned(),
            spec_slot: 6,
            spawn_flag: 1,
            attach_flag: 0,
            scale: 1.25,
            spec: vec![1; ENEMY_SPEC_BYTES],
        }),
        Message::PartnerBook(true),
        Message::BlockMoved(BlockMoved {
            phase: 2,
            id: 5,
            position: pos(),
        }),
        Message::BlockHeld(BlockHeld {
            phase: 2,
            id: 5,
            held: true,
        }),
        Message::DummyHit(story_call()),
        Message::WatchedCallbackFired(callback_fired()),
        Message::SharedFunctionRan(SharedFunctionRan {
            phase: 2,
            name: "shared".to_owned(),
        }),
        Message::MessageHandlerRan(MessageHandlerRan {
            phase: 2,
            name: "handler".to_owned(),
        }),
        Message::FightStream(FightStreamEntry {
            phase: 2,
            epoch: 3,
            seq: 0x1_0000_0001,
            name: "step".to_owned(),
            args: fight_args(),
        }),
        Message::FightWindow(FightWindowMark {
            phase: 2,
            epoch: 3,
            number: 4,
            part: 5,
            change: FightWindowChange::Opened {
                armed_by: Some("arm".to_owned()),
            },
        }),
        Message::FightRequest(FightRequest {
            phase: 2,
            epoch: 3,
            kind: FightRequestKind::RunPop {
                name: "pop".to_owned(),
                args: fight_args(),
                copy_number: 2,
            },
        }),
        Message::FightTape(FightTape {
            phase: 2,
            epoch: 3,
            seq: 9,
            native: "native".to_owned(),
            call_index: 4,
            value: FightTapeValue::Number(2.5),
        }),
        Message::FightRequestAnswer(FightRequestAnswer {
            phase: 2,
            epoch: 3,
            name: "pop".to_owned(),
            copy_number: 2,
            outcome: FightRequestOutcome::Refused {
                reason: FightRequestRefusal::Full,
            },
        }),
        Message::FightRadioWait(FightRadioWaitEnded {
            phase: 2,
            epoch: 3,
            seq: 9,
            wait_index: 1,
            host_waited_ms: 800,
        }),
        Message::FightPromptCount(FightPromptCount {
            phase: 2,
            kind: 1,
            id: 2,
            page: 3,
            dismissed: 4,
            native: 5,
            in_fight: true,
        }),
        Message::AttackGauge(AttackGaugeState {
            phase: 2,
            serial: 6,
            open: Some(gauge_open()),
        }),
        Message::AttackGaugeHit(AttackGaugeHit {
            phase: 2,
            serial: 6,
            node: 0,
            damage: 25,
            seq: 3,
        }),
        Message::GatherForScene(GatherForScene {
            phase: 2,
            position: pos(),
            in_side_view: true,
            side_view_door: 4,
        }),
        Message::GatherAnswer(GatherAnswer {
            phase: 2,
            reply: GatherReply::DidNotArrive,
        }),
        Message::ChoiceHover(ChoiceHover {
            talk_id: 41,
            line: 2,
            index: 1,
        }),
        Message::LobbyCharacter(LobbyCharacter { index: 2 }),
        Message::WallBroken(WallBroken {
            phase: 2,
            layout_id: Some(14),
            position: pos(),
        }),
        Message::BombBlast(BombBlast {
            phase: 2,
            position: pos(),
        }),
        Message::CartRide(CartRide {
            phase: 2,
            layout_id: Some(9),
            cart_position: pos(),
            from_state: 1,
            to_state: 2,
            boarded_at: [4.0, 5.0, 6.0],
        }),
        Message::LiftCall(LiftCall {
            phase: 2,
            point: 3,
            origin: 4,
            position: pos(),
        }),
        Message::LiftCallAnswer(LiftCallAnswer {
            phase: 2,
            point: 3,
            outcome: LiftCallOutcome::OtherMap,
        }),
        Message::FightCamera(FightCamera {
            phase: 2,
            seq: 3,
            natives_before: 4,
            camera: 5,
            blend_seconds: 0.5,
            blend_kind: 1,
            pose: Some(fight_camera_pose()),
        }),
        Message::EnemyIdList(EnemyIdList {
            phase: 2,
            tick: 8,
            part: 0,
            parts: 1,
            ids: vec![enemy_id()],
        }),
        Message::Member(Member {
            slot: 1,
            name: "Kaine".to_owned(),
        }),
        Message::MemberLeft { slot: 1 },
        Message::Rules(session_rules()),
        Message::MemberPings(vec![MemberPing {
            slot: 1,
            rtt_ms: 42,
        }]),
        Message::CampaignCard(CampaignCard {
            campaign_id: "camp".to_owned(),
            host: save_summary(),
            second_seat: Some(SecondSeatCard {
                summary: save_summary(),
                body: Some(PartnerBody::SeedKaineE),
            }),
        }),
        Message::SeatPart(SeatPart {
            transfer: 1,
            part: 0,
            parts: 1,
            total_bytes: 3,
            checksum: 0xCAFE_F00D,
            bytes: vec![1, 2, 3],
        }),
        Message::SeatReceipt(SeatReceipt {
            transfer: 1,
            outcome: SeatOutcome::Refused(SeatRefusal::BadChecksum),
        }),
        Message::GameSaved { succeeded: true },
        Message::CartRun(CartRun {
            phase: 2,
            layout_id: Some(9),
            cart_position: pos(),
            run_state: MIN_RUNNING_CART_STATE,
            seq: 7,
            progress: CartRunProgress::Moving {
                frame: 10.5,
                rate: 0.5,
            },
        }),
        Message::FightCameraAim(FightCameraAim {
            phase: 2,
            seq: 3,
            pose: fight_camera_pose(),
            follows_body: Some(FightCameraBodyOffset {
                eye_offset_xz: [1.0, 2.0],
                eye_height: 3.0,
                look_offset_xz: [4.0, 5.0],
                look_height: 6.0,
            }),
        }),
        Message::ShotCue(ShotCue {
            instance: Some(4),
            kind: ShotCueKind::LastingSound,
            id: 60,
            transform: [0.5; BULLET_TRANSFORM_FLOATS],
            offset: pos(),
            style: [0.25; BULLET_STYLE_FLOATS],
            sound_levels: [0.5, 0.75],
        }),
        Message::AttackGaugeAsk(AttackGaugeAsk {
            phase: 2,
            ask: 5,
            open: gauge_open(),
        }),
        Message::StandInTrapFired(callback_fired()),
        Message::SkipPromptAnswer(SkipPromptAnswer {
            phase: 2,
            prompt: 3,
            skip: true,
        }),
        Message::PartPickup(PartPickup {
            phase: 2,
            actor_handler_rva: 0x00AB_CDEF,
            part_group: 6,
        }),
        Message::SkipPromptVerdict(SkipPromptVerdict {
            phase: 2,
            prompt: 3,
            skip: false,
        }),
        Message::RuleRoomBreak(RuleRoomBreak {
            phase: 2,
            handler: "room".to_owned(),
        }),
        Message::WorldObjectMoved(WorldObjectMoved {
            phase: 2,
            key: world_object_key(),
            position: pos(),
            held: true,
        }),
        Message::WorldObjectBroken(WorldObjectBroken {
            phase: 2,
            key: world_object_key(),
        }),
    ]
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn golden_lines() -> Vec<(&'static str, &'static str)> {
    GOLDEN_HEX
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.split_once(' ').expect("a golden line is `Name hex`"))
        .collect()
}

#[test]
fn the_protocol_number_is_pinned_here_and_nowhere_else() {
    assert_eq!(PROTOCOL_VERSION, 74);
}

#[test]
fn every_message_variant_keeps_its_encoded_bytes() {
    let golden = golden_lines();
    let samples = samples();
    for (index, message) in samples.iter().enumerate() {
        let name = message.kind_name();
        let actual = hex_of(&encode(message).expect("a sample encodes"));
        let Some((golden_name, golden_hex)) = golden.get(index) else {
            panic!(
                "variant {name} (sample {index}) has no line in golden_wire.hex: `{name} {actual}`"
            );
        };
        assert_eq!(
            *golden_name, name,
            "golden line {index} names another variant"
        );
        assert_eq!(
            *golden_hex, actual,
            "the encoding of variant {name} changed"
        );
    }
    assert_eq!(
        golden.len(),
        samples.len(),
        "golden_wire.hex has lines for variants that have no sample"
    );
}

#[test]
fn samples_are_the_variants_in_declaration_order() {
    for (index, message) in samples().iter().enumerate() {
        let bytes = encode(message).expect("a sample encodes");
        assert_eq!(
            usize::from(bytes[0]),
            index,
            "sample {index} is variant {}, so it is out of declaration order",
            message.kind_name()
        );
    }
}

#[test]
fn every_variant_has_a_sample() {
    let next_index = u8::try_from(samples().len()).expect("fewer than 128 variants");
    let beyond_last = postcard::from_bytes::<Message>(&[next_index]);
    assert!(
        matches!(beyond_last, Err(postcard::Error::SerdeDeCustom)),
        "Message has a variant after sample {next_index}: add its sample and its golden line"
    );
}

#[test]
fn every_golden_sample_decodes_back_to_itself() {
    for message in samples() {
        let bytes = encode(&message).expect("a sample encodes");
        assert_eq!(decode(&bytes), Ok(message));
    }
}

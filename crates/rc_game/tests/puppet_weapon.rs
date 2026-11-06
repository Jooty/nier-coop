use rc_game::offsets::{
    ACTOR_DRAW_MASK_OFFSET, ACTOR_DRAWN_FLAG_OFFSET, ENTITY_RVA, PLAYER_ATTACH_TABLE_OFFSETS,
    PLAYER_WEAPON_ALLOWED_FLAG_OFFSET, PLAYER_WEAPON_IN_HAND_FLAG_OFFSET,
    SAVE_EQUIPPED_WEAPON_ID_OFFSET, SAVE_FORCED_WEAPON_FLAG_OFFSET, SAVE_WEAPON_LEVELS_OFFSET,
    WEAPON_IN_HAND_ATTACH_ID, WEAPON_ON_BACK_ATTACH_ID, WEAPON_RECORD_GRIP_IN_HAND_OFFSET,
    WEAPON_TABLE_RVA,
};
use rc_game::puppet::{
    Affine, AttachPoint, PuppetPose, read_attach_point, read_weapon_grip, read_weapon_look,
};
use rc_game::{GameMemory, Position};
use rc_proto::WeaponState;

use crate::fake_memory::FakeMemory;

const PLAYER: u64 = 0x2_0000_0000;
const SHARED_TABLE: u64 = 0x5_0000_0000;
const RECORD: u64 = 0x6_0000_0000;

fn close(a: [f32; 3], b: [f32; 3]) -> bool {
    a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4)
}

fn point_through(affine: &Affine, point: [f32; 3]) -> [f32; 3] {
    affine.then(&Affine::translation(point)).position()
}

fn save_with_weapon(id: i32, level: i8) -> FakeMemory {
    let mut memory = FakeMemory::new();
    let save = u64::from(ENTITY_RVA);
    memory.put_at_rva(
        (save + SAVE_EQUIPPED_WEAPON_ID_OFFSET) as u32,
        &id.to_le_bytes(),
    );
    memory.put_at_rva(
        (save + SAVE_FORCED_WEAPON_FLAG_OFFSET) as u32,
        &0_i32.to_le_bytes(),
    );
    let mut levels = [-1_i8 as u8; 64];
    if let Ok(slot) = usize::try_from(id)
        && slot < levels.len()
    {
        levels[slot] = level as u8;
    }
    memory.put_at_rva((save + SAVE_WEAPON_LEVELS_OFFSET) as u32, &levels);
    memory.put(PLAYER + ACTOR_DRAWN_FLAG_OFFSET, &[1]);
    memory.put(PLAYER + ACTOR_DRAW_MASK_OFFSET, &[0xFF]);
    memory.put(PLAYER + PLAYER_WEAPON_ALLOWED_FLAG_OFFSET, &[1]);
    memory.put(PLAYER + PLAYER_WEAPON_IN_HAND_FLAG_OFFSET, &[1]);
    memory
}

#[test]
fn the_equipped_weapon_is_read_with_its_level_and_hand() {
    let memory = save_with_weapon(5, 2);
    assert_eq!(
        read_weapon_look(&memory, PLAYER),
        Ok(Some(WeaponState {
            id: 5,
            level: 2,
            in_hand: true,
            visible: true,
        }))
    );
}

#[test]
fn a_weapon_the_player_hides_is_sent_as_not_visible() {
    let mut memory = save_with_weapon(5, 0);
    memory.put(PLAYER + ACTOR_DRAW_MASK_OFFSET, &[0x07]);
    let look = read_weapon_look(&memory, PLAYER)
        .expect("reads")
        .expect("a weapon");
    assert!(!look.visible);
}

#[test]
fn the_forced_weapon_flag_wins_over_the_equipped_id() {
    let mut memory = save_with_weapon(5, 0);
    memory.put_at_rva(
        (u64::from(ENTITY_RVA) + SAVE_FORCED_WEAPON_FLAG_OFFSET) as u32,
        &1_i32.to_le_bytes(),
    );
    let look = read_weapon_look(&memory, PLAYER)
        .expect("reads")
        .expect("a weapon");
    assert_eq!(look.id, 0xE);
}

#[test]
fn no_weapon_or_an_unowned_level_is_handled() {
    assert_eq!(read_weapon_look(&save_with_weapon(-1, 0), PLAYER), Ok(None));
    assert_eq!(read_weapon_look(&save_with_weapon(64, 0), PLAYER), Ok(None));
    let unowned = save_with_weapon(3, -1);
    let look = read_weapon_look(&unowned, PLAYER)
        .expect("reads")
        .expect("a weapon");
    assert_eq!(look.level, 0);
}

#[test]
fn the_grip_comes_from_the_weapon_record() {
    let mut memory = FakeMemory::new();
    memory.put_at_rva(WEAPON_TABLE_RVA + 7 * 8, &RECORD.to_le_bytes());
    let grip: Vec<u8> = [0.1_f32, -0.2, 0.3]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    memory.put(RECORD + WEAPON_RECORD_GRIP_IN_HAND_OFFSET, &grip);
    assert_eq!(
        read_weapon_grip(&memory, 7, true),
        Ok(Some([0.1, -0.2, 0.3]))
    );
    memory.put_at_rva(WEAPON_TABLE_RVA + 8 * 8, &0_u64.to_le_bytes());
    assert_eq!(read_weapon_grip(&memory, 8, true), Ok(None));
    assert_eq!(read_weapon_grip(&memory, 64, true), Ok(None));
}

fn put_attach_entry(memory: &mut FakeMemory, entry: u64, id: i32, name: &str, root: bool) {
    let name_at = entry + 0x100;
    memory.put(entry, &id.to_le_bytes());
    memory.put(entry + 4, &((name_at - (entry + 4)) as i32).to_le_bytes());
    let floats: Vec<u8> = [0.5_f32, 0.0, 0.0, 0.0, 90.0, 0.0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    memory.put(entry + 8, &floats);
    memory.put(entry + 0x20, &[u8::from(root), 0, 0, 0]);
    let mut bytes = name.as_bytes().to_vec();
    bytes.resize(64, 0);
    memory.put(name_at, &bytes);
}

#[test]
fn an_attach_point_missing_from_the_player_table_is_found_in_the_shared_one() {
    let mut memory = FakeMemory::new();
    memory.put(
        PLAYER + PLAYER_ATTACH_TABLE_OFFSETS[0],
        &0_u64.to_le_bytes(),
    );
    memory.put(
        PLAYER + PLAYER_ATTACH_TABLE_OFFSETS[1],
        &SHARED_TABLE.to_le_bytes(),
    );
    memory.put(SHARED_TABLE, &2_i32.to_le_bytes());
    memory.put(SHARED_TABLE + 4, &0x10_i32.to_le_bytes());
    let entries = SHARED_TABLE + 4 + 0x10;
    put_attach_entry(
        &mut memory,
        entries,
        WEAPON_ON_BACK_ATTACH_ID,
        "Spine2",
        false,
    );
    put_attach_entry(
        &mut memory,
        entries + 36,
        WEAPON_IN_HAND_ATTACH_ID,
        "RightHand",
        true,
    );
    let point = read_attach_point(&memory, PLAYER, WEAPON_IN_HAND_ATTACH_ID)
        .expect("reads")
        .expect("found");
    assert_eq!(point.bone, "RightHand");
    assert!(point.keeps_root_rotation);
    assert!(close(point.translation, [0.5, 0.0, 0.0]));
    assert!(close(point.rotation_degrees, [0.0, 90.0, 0.0]));
    assert_eq!(read_attach_point(&memory, PLAYER, 999), Ok(None));
}

#[test]
fn euler_angles_follow_the_game_order() {
    let quarter = core::f32::consts::FRAC_PI_2;
    let about_y = Affine::rotation_xyz([0.0, quarter, 0.0]);
    assert!(close(
        point_through(&about_y, [1.0, 0.0, 0.0]),
        [0.0, 0.0, -1.0]
    ));
    let z_then_x = Affine::rotation_xyz([quarter, 0.0, quarter]);
    assert!(close(
        point_through(&z_then_x, [0.0, 1.0, 0.0]),
        [0.0, 0.0, 1.0]
    ));
}

#[test]
fn the_attach_frame_offsets_along_the_bone_and_can_keep_the_root_turn() {
    let quarter = core::f32::consts::FRAC_PI_2;
    let mut bone = Affine::rotation_xyz([0.0, 0.0, quarter]);
    bone.rows[0][3] = 10.0;
    let root = Affine::IDENTITY;
    let point = AttachPoint {
        bone: "RightHand".to_owned(),
        translation: [1.0, 0.0, 0.0],
        rotation_degrees: [0.0; 3],
        keeps_root_rotation: false,
        target: 0,
    };
    assert!(close(
        point.frame(&bone, &root).position(),
        [10.0, 1.0, 0.0]
    ));
    let rooted = AttachPoint {
        keeps_root_rotation: true,
        ..point
    };
    assert!(close(
        rooted.frame(&bone, &root).position(),
        [11.0, 0.0, 0.0]
    ));
}

#[test]
fn the_matrix_bytes_round_trip_through_the_transform_block() {
    let pose = PuppetPose {
        position: Position {
            x: 1.0,
            y: 2.0,
            z: 3.0,
        },
        yaw: 0.7,
    };
    let bytes = pose.creation_matrix();
    let block: [u8; 48] = bytes[..48].try_into().expect("48 bytes");
    let affine = Affine::from_block(&block);
    assert_eq!(affine.to_matrix_bytes(), bytes);
    assert!(close(affine.position(), [1.0, 2.0, 3.0]));
}

#[test]
fn reading_an_unmapped_player_is_an_error_not_a_panic() {
    let memory = FakeMemory::new();
    assert!(read_attach_point(&memory, PLAYER, WEAPON_IN_HAND_ATTACH_ID).is_err());
    assert!(memory.read_u8(PLAYER).is_err());
}

use crate::blocks::distance_between;
use crate::kaine::COMPANION_PLACE_TRUSTED_WITHIN_UNITS;
use crate::puppet::PuppetPose;

/// A story scene in which the guest is put where the companion stands for it: the host's talk
/// `talk_id` opens it, and the talk's end, `ended_by` or the host's flag `closed_by_game_flag`
/// closes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneStandIn {
    pub scene: &'static str,
    pub pack: &'static str,
    pub talk_id: u32,
    pub ended_by: &'static [&'static str],
    pub closed_by_game_flag: u16,
    pub script_spot: [f32; 3],
    /// As the script hands it to `_SetUniqueActorPosition`; her actor's yaw in radians is then
    /// minus this in degrees, and `_SetPlayerPosition` is given it as the stand-in beats are.
    pub script_heading_degrees: f32,
    /// Script functions of the scene that use her real actor, so she is not switched off for it.
    pub companion_kept_for: &'static [&'static str],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScenePlaceSource {
    HerActor,
    Script,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScenePlace {
    pub spot: [f32; 3],
    pub heading_degrees: f32,
    pub source: ScenePlaceSource,
}

pub const SCENE_STAND_INS: [SceneStandIn; 1] = [SceneStandIn {
    scene: "gate talk",
    pack: "A_NORTH_FIELD_01",
    talk_id: 1207,
    ended_by: &["EID_1050_a0088g_End"],
    closed_by_game_flag: 389,
    script_spot: [153.85, 9.57, 148.89],
    script_heading_degrees: 4.0,
    companion_kept_for: &[
        "EID_1050_a0088g_Start",
        "EID_1050_a0088g_Circular_Camera",
        "EID_1050_a0088g_Zoom_Camera",
    ],
}];

pub fn scene_stand_in_for_talk(talk_id: u32) -> Option<&'static SceneStandIn> {
    SCENE_STAND_INS.iter().find(|row| row.talk_id == talk_id)
}

/// Where the guest goes for `row`: her actor's place and facing when it is near the script's
/// spot (the script has put her there), else the script's own numbers.
pub fn scene_stand_in_place(row: &SceneStandIn, her: Option<PuppetPose>) -> ScenePlace {
    let trusted = her.filter(|pose| {
        let at = [pose.position.x, pose.position.y, pose.position.z];
        pose.yaw.is_finite()
            && at.iter().all(|value| value.is_finite())
            && distance_between(at, row.script_spot) <= COMPANION_PLACE_TRUSTED_WITHIN_UNITS
    });
    match trusted {
        Some(pose) => ScenePlace {
            spot: [pose.position.x, pose.position.y, pose.position.z],
            heading_degrees: -pose.yaw.to_degrees(),
            source: ScenePlaceSource::HerActor,
        },
        None => ScenePlace {
            spot: row.script_spot,
            heading_degrees: row.script_heading_degrees,
            source: ScenePlaceSource::Script,
        },
    }
}

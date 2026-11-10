mod event_callback;
mod kind;
mod lookup;
mod place;
mod scene_scope;
mod table;

pub use event_callback::{is_event_callback_name, is_event_id_name};
pub use kind::StoryCallKind;
pub use lookup::{kind_in_table, name_hash, story_call_kind, story_call_kind_by_name};
pub use place::{PartnerPlace, Whereabouts};
pub use scene_scope::SceneScope;
pub use table::STORY_CALL_TABLE;

pub const LIBRARY_PACK_HASH: u64 = 0x7588_BE80_E701_6100;
pub const SAME_PLACE_RANGE_UNITS: f32 = 30.0;
pub const LOCAL_SCENE_NAMES: [&str; 3] =
    ["PlayerActionDoor", "PlayerCloseDoor", "Reset_Camera_Flag"];
pub const LOCAL_SCENE_NAME_PREFIXES: [&str; 16] = [
    "GM_OUT_",
    "GM_IN_",
    "CB_",
    "LOOK_AT_",
    "Gaze_to_",
    "From_1F_to_",
    "From_2F_to_",
    "Change_Bright",
    "WaterHeight_",
    "Stop_Env_",
    "Play_Env_",
    "PLAY_SE_ENV",
    "LIB_SNOW_",
    "Talk_Text_",
    "door_",
    "knock_door_",
];
pub const LOCAL_SCENE_NAME_PARTS_LOWERCASE: [&str; 2] = ["camera", "padshake"];

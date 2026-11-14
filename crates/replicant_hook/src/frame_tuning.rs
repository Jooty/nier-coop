mod store;

pub use store::{frame_tuning, reset_frame_tuning, save_frame_tuning, set_frame_tuning};

pub(crate) use store::load_frame_tuning;

const TUNING_FILE_NAME: &str = "niertogether-party-frame.toml";
const OLD_TUNING_FILE_NAMES: [&str; 2] = [
    "nier-coop-party-frame.toml",
    "replicant-coop-party-frame.toml",
];

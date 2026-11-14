use std::sync::Arc;

use nt_net::{IrohConfig, IrohTransport, NetError, Transport};
use replicant_proto::{BuildGate, MAX_DISPLAY_NAME_BYTES};

use crate::DEFAULT_PLAYER_NAME;
use crate::bridge::GameWire;
use crate::profile::{GameProfile, default_profile};

pub type TransportFactory = Arc<dyn Fn() -> Result<Box<dyn Transport>, NetError> + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherConfig {
    pub name: String,
    pub build: BuildGate,
}

impl LauncherConfig {
    pub fn new(name: &str) -> Self {
        let build = default_profile().supported_exe_builds[0];
        Self {
            name: clean_name(name),
            build: BuildGate {
                time_date_stamp: build.time_date_stamp,
                size_of_image: build.size_of_image,
            },
        }
    }

    pub fn for_game(profile: &GameProfile, name: &str) -> Self {
        Self {
            name: clean_name(name),
            build: GameWire::of(profile).session_build(),
        }
    }
}

pub fn iroh_factory(config: IrohConfig) -> TransportFactory {
    Arc::new(move || Ok(Box::new(IrohTransport::new(config)?) as Box<dyn Transport>))
}

fn clean_name(name: &str) -> String {
    let mut cleaned: String = name.chars().filter(|c| !c.is_control()).collect();
    cleaned = cleaned.trim().to_owned();
    while cleaned.len() > MAX_DISPLAY_NAME_BYTES {
        cleaned.pop();
    }
    if cleaned.is_empty() {
        DEFAULT_PLAYER_NAME.to_owned()
    } else {
        cleaned
    }
}

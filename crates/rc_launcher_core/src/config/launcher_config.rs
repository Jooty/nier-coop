use std::sync::Arc;

use rc_game::build::SUPPORTED_PE_IDENTITY;
use rc_net::{IrohConfig, IrohTransport, NetError, Transport};
use rc_proto::{BuildGate, MAX_DISPLAY_NAME_BYTES};

use crate::DEFAULT_PLAYER_NAME;

pub type TransportFactory = Arc<dyn Fn() -> Result<Box<dyn Transport>, NetError> + Send + Sync>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LauncherConfig {
    pub name: String,
    pub build: BuildGate,
}

impl LauncherConfig {
    pub fn new(name: &str) -> Self {
        Self {
            name: clean_name(name),
            build: BuildGate {
                time_date_stamp: SUPPORTED_PE_IDENTITY.time_date_stamp,
                size_of_image: SUPPORTED_PE_IDENTITY.size_of_image,
            },
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

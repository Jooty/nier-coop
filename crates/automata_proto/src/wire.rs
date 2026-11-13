mod character_hello;
mod codec;
mod message;
mod path_info;
mod player_state;

pub use automata_game::enemies::{
    EnemyHit, EnemyHits, EnemyId, EnemyRosterPart, EnemySighting, EnemySpawn, GeneratorPhase,
    GeneratorState, GeneratorStates, LiveGenerators,
};
pub use automata_game::enemy_lead::{EnemyLead, EnemyPose, LeadTarget};
pub use automata_game::load_gate::{LoadSignal, StoryPlace};
pub use automata_game::pickups::ItemPickup;
pub use automata_game::travel::TeleportRequest;
pub use character_hello::CharacterHello;
pub use codec::{decode, encode};
pub use message::Message;
pub use nc_wire::{
    BuildGate, GameReadiness, Hello, LobbyCharacter, LobbyReady, LobbyStart, MAX_MESSAGE_BYTES,
    Member, Ping, Pong, Reject, RejectReason, Welcome, WireError,
};
pub use path_info::PathInfo;
pub use player_state::PlayerState;

pub const PROTOCOL_VERSION: u16 = 1023;
pub const MAX_PLAYERS: u8 = 16;
pub const MAX_DISPLAY_NAME_BYTES: usize = 32;
pub const MAX_POSITION_ABS: f32 = 1_000_000.0;
pub const MAX_FACING_ABS_RADIANS: f32 = 16.0;
pub const MAX_SPEED: f32 = automata_game::partner::HIGHEST_SPEED_SENT;
pub const MAX_STATE_ID: u16 = automata_game::partner::HIGHEST_STATE_ID;

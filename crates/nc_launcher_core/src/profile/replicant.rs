use crate::profile::{Character, GameProfile, GameSupport};

const REPLICANT_CHARACTERS: [Character; 2] = [
    Character {
        id: "nier",
        label: "Nier (follows the story)",
    },
    Character {
        id: "prologue",
        label: "The prologue character",
    },
];

pub const REPLICANT: GameProfile = GameProfile {
    game_id: "replicant",
    display_name: "NieR Replicant",
    steam_app_id: "1113560",
    install_folder_name: "NieR Replicant ver.1.22474487139",
    exe_names: &["NieR Replicant ver.1.22474487139.exe"],
    loader_dll_name: "xinput9_1_0.dll",
    hook_dll_name: "replicant_hook.dll",
    link_file_name: "nier-coop.toml",
    characters: &REPLICANT_CHARACTERS,
    characters_shareable: true,
    support: GameSupport {
        install: true,
        start: true,
        saves: true,
    },
};

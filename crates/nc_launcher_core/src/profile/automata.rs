use crate::profile::{Character, GameProfile, GameSupport};

const AUTOMATA_CHARACTERS: [Character; 3] = [
    Character {
        id: "2b",
        label: "2B",
    },
    Character {
        id: "9s",
        label: "9S",
    },
    Character {
        id: "a2",
        label: "A2",
    },
];

pub const AUTOMATA: GameProfile = GameProfile {
    game_id: "automata",
    display_name: "NieR:Automata",
    steam_app_id: "524220",
    install_folder_name: "NieRAutomata",
    exe_names: &["NieRAutomata.exe", "NieRAutomataCompat.exe"],
    loader_dll_name: "dinput8.dll",
    hook_dll_name: "automata_hook.dll",
    link_file_name: "nier-coop.toml",
    characters: &AUTOMATA_CHARACTERS,
    characters_shareable: false,
    support: GameSupport {
        install: false,
        start: false,
        saves: false,
    },
};

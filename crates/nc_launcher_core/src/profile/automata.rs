use crate::profile::{
    CO_OP_CAMPAIGN, Character, CharacterArt, FREE_ROAM, GameMode, GameProfile, GameSupport,
    HIDE_AND_SEEK, PROP_HUNT,
};

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

const AUTOMATA_CHARACTER_ART: [CharacterArt; 3] = [
    CharacterArt {
        character_id: "2b",
        focus_x_per_mille: 578,
        focus_y_per_mille: 500,
        height_share_per_mille: 760,
    },
    CharacterArt {
        character_id: "9s",
        focus_x_per_mille: 440,
        focus_y_per_mille: 640,
        height_share_per_mille: 560,
    },
    CharacterArt {
        character_id: "a2",
        focus_x_per_mille: 697,
        focus_y_per_mille: 320,
        height_share_per_mille: 400,
    },
];

const AUTOMATA_MODES: [GameMode; 4] = [CO_OP_CAMPAIGN, FREE_ROAM, HIDE_AND_SEEK, PROP_HUNT];

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
    character_art: &AUTOMATA_CHARACTER_ART,
    modes: &AUTOMATA_MODES,
    support: GameSupport {
        install: true,
        start: true,
        saves: false,
    },
};

use automata_game::build::SUPPORTED_PE_IDENTITIES;

use crate::profile::{
    CO_OP_CAMPAIGN, Character, CharacterArt, FREE_ROAM, FreeRoamMap, GameMode, GameProfile,
    GameSupport, HIDE_AND_SEEK, PROP_HUNT,
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

const AUTOMATA_FREE_ROAM_MAPS: [FreeRoamMap; 5] = [
    FreeRoamMap {
        id: "city-ruins",
        label: "City Ruins",
        start_index: 0,
    },
    FreeRoamMap {
        id: "desert",
        label: "Desert",
        start_index: 1,
    },
    FreeRoamMap {
        id: "amusement-park",
        label: "Amusement Park",
        start_index: 2,
    },
    FreeRoamMap {
        id: "pascals-village",
        label: "Pascal's Village",
        start_index: 3,
    },
    FreeRoamMap {
        id: "flooded-city",
        label: "Flooded City",
        start_index: 5,
    },
];

pub const AUTOMATA: GameProfile = GameProfile {
    game_id: "automata",
    display_name: "NieR:Automata",
    steam_app_id: "524220",
    install_folder_name: "NieRAutomata",
    exe_names: &["NieRAutomata.exe", "NieRAutomataCompat.exe"],
    supported_exe_builds: &SUPPORTED_PE_IDENTITIES,
    loader_dll_name: "dinput8.dll",
    hook_dll_name: "automata_hook.dll",
    link_file_name: "nier-coop.toml",
    characters: &AUTOMATA_CHARACTERS,
    characters_shareable: false,
    character_art: &AUTOMATA_CHARACTER_ART,
    modes: &AUTOMATA_MODES,
    free_roam_maps: &AUTOMATA_FREE_ROAM_MAPS,
    support: GameSupport {
        install: true,
        start: true,
        saves: false,
        hook_runs_the_campaign_start: true,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Character {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameSupport {
    pub install: bool,
    pub start: bool,
    pub saves: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameProfile {
    pub game_id: &'static str,
    pub display_name: &'static str,
    pub steam_app_id: &'static str,
    pub install_folder_name: &'static str,
    pub exe_names: &'static [&'static str],
    pub loader_dll_name: &'static str,
    pub hook_dll_name: &'static str,
    pub link_file_name: &'static str,
    pub characters: &'static [Character],
    pub characters_shareable: bool,
    pub support: GameSupport,
}

const DLL_SUFFIX: &str = ".dll";

impl GameProfile {
    pub fn loader_dll_stem(&self) -> &'static str {
        self.loader_dll_name
            .strip_suffix(DLL_SUFFIX)
            .unwrap_or(self.loader_dll_name)
    }

    pub fn is_game_exe(&self, file_name: &str) -> bool {
        self.exe_names.contains(&file_name)
    }

    pub fn supports_anything(&self) -> bool {
        self.support.install || self.support.start || self.support.saves
    }

    pub fn character(&self, id: &str) -> Option<&'static Character> {
        self.characters.iter().find(|character| character.id == id)
    }
}

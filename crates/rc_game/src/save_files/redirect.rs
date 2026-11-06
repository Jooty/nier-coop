use crate::save_files::GAMEDATA_FILE_NAME;

const COOP_GAMEDATA_C_NAME: &[u8] = b"GAMEDATA.coop\0";

pub fn coop_save_name(game_file_name: &[u8]) -> Option<&'static [u8]> {
    (game_file_name == GAMEDATA_FILE_NAME.as_bytes()).then_some(COOP_GAMEDATA_C_NAME)
}

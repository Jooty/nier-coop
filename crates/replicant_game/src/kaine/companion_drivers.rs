use crate::kaine::functions_that_drive_her_from_scripts::FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS;

pub const DRIVER_ROW_PACK_FOR_A_LIBRARY_FUNCTION: &str = "*";

/// Whether `name`, run in `pack`, starts the companion's script mode or gives her a script
/// action: she must be active for the action and its callback to run.
pub fn function_drives_her(pack: &str, name: &str) -> bool {
    [pack, DRIVER_ROW_PACK_FOR_A_LIBRARY_FUNCTION]
        .into_iter()
        .any(|row_pack| {
            FUNCTIONS_THAT_DRIVE_HER_FROM_SCRIPTS
                .binary_search(&(row_pack, name))
                .is_ok()
        })
}

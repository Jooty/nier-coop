use std::process::Command;

use crate::second_copy::{
    CopyCommand, copy_command, copy_environment, proton_script_from_config_info,
};
use crate::steam::{GameInstall, hide_console_window};

pub fn start_second_copy(
    game: &GameInstall,
    link_port: u16,
    name: &str,
    boot_slot: Option<u8>,
) -> Result<(), String> {
    let proton_script = if cfg!(windows) {
        None
    } else {
        let config_info = std::fs::read_to_string(game.proton_compat_dir().join("config_info"))
            .map_err(|_| {
                "could not find the Proton build; start the game once from Steam".to_owned()
            })?;
        Some(
            proton_script_from_config_info(&config_info)
                .ok_or("config_info names no Proton build")?,
        )
    };
    let command = copy_command(
        game,
        copy_environment(link_port, name, boot_slot),
        proton_script.as_deref(),
    );
    spawn(&command).map_err(|error| error.to_string())
}

fn spawn(command: &CopyCommand) -> std::io::Result<()> {
    let mut process = Command::new(&command.program);
    process
        .args(&command.args)
        .envs(command.envs.iter().map(|(key, value)| (key, value)))
        .current_dir(&command.current_dir);
    hide_console_window(&mut process);
    process.spawn().map(|_| ())
}

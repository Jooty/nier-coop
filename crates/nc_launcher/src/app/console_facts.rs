use nc_launcher_core::{
    ConsoleGameFacts, ConsoleSave, ConsoleScript, ConsoleTask, HostingFacts, JoiningFacts,
    hosting_script, joining_script, ticket_check,
};
use replicant_proto::PROTOCOL_VERSION;

use crate::app::{Launcher, NetForms};
use crate::home::folder_display;
use crate::install::ModStatus;
use crate::net::LOBBY_MODE_CONTINUE;

/// The boot console's script for the wait under way, from the launcher's own states.
pub fn console_script(launcher: &Launcher, forms: &NetForms, task: ConsoleTask) -> ConsoleScript {
    let state = forms.net.status().state;
    let player_name = launcher.own_seat_name().name;
    let card = launcher
        .cards
        .iter()
        .find(|card| card.profile.game_id == launcher.profile.game_id);
    let installed_build = card.and_then(|card| card.installed_build.clone());
    let waiting_update = match card {
        Some(card) if card.mod_status == ModStatus::Outdated => card.ready_build.clone(),
        Some(card) if card.mod_status == ModStatus::Current => {
            launcher.server_build_for(card.profile.game_id)
        }
        _ => None,
    };
    let game = ConsoleGameFacts {
        protocol: PROTOCOL_VERSION,
        installed_build: installed_build.as_deref(),
        waiting_update: waiting_update.as_deref(),
        player_name: &player_name,
    };
    match task {
        ConsoleTask::Hosting => {
            let (folder, mod_files) = launcher.hosting_checks();
            let folder_text = launcher.game.as_ref().map(|game| {
                // The console sets each fact on one line.
                folder_display(&game.game_dir, dirs::home_dir().as_deref()).replace('\n', "")
            });
            let executable = launcher.game.as_ref().and_then(|game| {
                game.exe_path()
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
            });
            let play_time = launcher
                .selected_save()
                .map(|slot| slot.play_time_text())
                .unwrap_or_default();
            let saved_game = if launcher.lobby_mode == LOBBY_MODE_CONTINUE {
                Some(launcher.selected_save().map(|slot| ConsoleSave {
                    slot: slot.index,
                    level: slot.level,
                    play_time: &play_time,
                }))
            } else {
                Some(None)
            };
            let answer_text = forms.host_result.as_ref().map(|answer| match answer {
                Ok(ticket) => Ok(ticket.clone()),
                Err(error) => Err(error.to_string()),
            });
            let options = forms.session_options;
            hosting_script(HostingFacts {
                game_folder: match (&folder, &folder_text) {
                    (Ok(()), Some(text)) => Ok(text.as_str()),
                    (Ok(()), None) => Ok(""),
                    (Err(error), _) => Err(error.as_str()),
                },
                executable: executable.as_deref(),
                mod_files: mod_files.as_ref().map(|_| ()).map_err(String::as_str),
                hook_file: launcher.profile.hook_dll_name,
                game,
                saved_game,
                seats: launcher.link.rules.max_players,
                enemy_health_percent: options.enemy_health_percent,
                enemy_damage_percent: options.enemy_damage_percent,
                host_answer: answer_text
                    .as_ref()
                    .map(|answer| answer.as_deref().map_err(String::as_str)),
                state: &state,
            })
        }
        ConsoleTask::Joining => {
            let probe_error = match &forms.probe_result {
                Some(Err(error)) => Some(error.to_string()),
                _ => None,
            };
            let join_error = match &forms.join_result {
                Some(Err(error)) => Some(error.to_string()),
                _ => None,
            };
            joining_script(JoiningFacts {
                ticket_text: &forms.join_ticket,
                ticket: ticket_check(&forms.join_ticket),
                game,
                probe: match &forms.probe_result {
                    Some(Ok(report)) => Some(Ok(report)),
                    Some(Err(_)) => probe_error.as_deref().map(Err),
                    None => None,
                },
                join_answer: match &forms.join_result {
                    Some(Ok(())) => Some(Ok(())),
                    Some(Err(_)) => join_error.as_deref().map(Err),
                    None => None,
                },
                state: &state,
                own_slot: launcher.link.own_slot,
            })
        }
    }
}

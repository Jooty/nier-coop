use nc_launcher_core::{
    ConsoleScript, ConsoleTask, HostingFacts, JoiningFacts, hosting_script, joining_script,
    ticket_check,
};

use crate::app::{Launcher, NetForms};

/// The boot console's script for the wait under way, from the launcher's own states.
pub fn console_script(launcher: &Launcher, forms: &NetForms, task: ConsoleTask) -> ConsoleScript {
    let state = forms.net.status().state;
    match task {
        ConsoleTask::Hosting => {
            let (folder, mod_files) = launcher.hosting_checks();
            let answer_text = forms.host_result.as_ref().map(|answer| match answer {
                Ok(ticket) => Ok(ticket.clone()),
                Err(error) => Err(error.to_string()),
            });
            hosting_script(HostingFacts {
                game_folder: folder.as_ref().map(|_| ()).map_err(String::as_str),
                mod_files: mod_files.as_ref().map(|_| ()).map_err(String::as_str),
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
                ticket: ticket_check(&forms.join_ticket),
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
            })
        }
    }
}

use std::sync::{Mutex, PoisonError};

use rc_game::GameMemory;
use rc_game::dialogue::{
    DialogueTracker, find_talk_id, partner_line_events, read_line_text, read_talk_cursor,
};
use rc_proto::{DialogueLine, DialogueUpdate, Message};

use crate::build_gate::VerifiedBuild;
use crate::diagnostics::log_line;
use crate::event_log::post_event_unlogged;
use crate::memory::InProcessMemory;
use crate::net::{self, NetShared, NetThread};

static TRACKER: Mutex<DialogueTracker> = Mutex::new(DialogueTracker::new());

pub fn tick_dialogue(build: &VerifiedBuild) {
    let Some(net) = net::handle() else {
        return;
    };
    let view = net.view();
    let memory = InProcessMemory::new(build);
    share_local_talk(net, &view, &memory);
    show_partner_lines(net, &view, &memory);
}

fn share_local_talk(net: &NetThread, view: &NetShared, memory: &InProcessMemory) {
    let cursor = read_talk_cursor(memory).ok().flatten();
    let update = TRACKER
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .observe(cursor, view.partner_in_play(), |talk_address| {
            let talk_id = find_talk_id(memory, talk_address).ok().flatten();
            if talk_id.is_none() {
                log_line("dialogue: the open talk is not in the talk table, not shared");
            }
            talk_id
        });
    let Some(update) = update else {
        return;
    };
    match update {
        DialogueUpdate::Line(line) => log_line(&format!(
            "dialogue: sharing talk {} line {}",
            line.talk_id, line.line
        )),
        DialogueUpdate::Ended => log_line("dialogue: sharing talk ended"),
    }
    net.send_relayed(Message::Dialogue(update));
}

fn show_partner_lines(net: &NetThread, view: &NetShared, memory: &InProcessMemory) {
    let arrived = net.take_relayed(|message| matches!(message, Message::Dialogue(_)));
    let partner_name = view
        .peer_name
        .clone()
        .unwrap_or_else(|| "Partner".to_owned());
    for message in arrived {
        match message {
            Message::Dialogue(DialogueUpdate::Line(line)) => {
                show_partner_line(memory, &partner_name, line);
            }
            Message::Dialogue(DialogueUpdate::Ended) => {
                log_line("dialogue: partner talk ended");
            }
            _ => {}
        }
    }
}

fn show_partner_line<M: GameMemory>(memory: &M, partner_name: &str, line: DialogueLine) {
    // Ids only in the file log: the line itself goes to the on-screen event log alone.
    match read_line_text(memory, line) {
        Ok(Some(text)) => {
            let entries = partner_line_events(partner_name, &text);
            log_line(&format!(
                "dialogue: partner talk {} line {} shown in {} event log entries",
                line.talk_id,
                line.line,
                entries.len()
            ));
            for entry in entries {
                post_event_unlogged(&entry);
            }
        }
        Ok(None) => log_line(&format!(
            "dialogue: partner talk {} line {} is not loaded here",
            line.talk_id, line.line
        )),
        Err(error) => log_line(&format!(
            "dialogue: partner talk {} line {} unreadable ({error})",
            line.talk_id, line.line
        )),
    }
}

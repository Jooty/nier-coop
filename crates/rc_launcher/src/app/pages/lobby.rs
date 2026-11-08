use eframe::egui::{self, Ui};

use crate::app::pages::host_options::draw_host_options;
use crate::app::pages::lobby_view::draw_player_list;
use crate::app::style::{BAD, GOOD, WARN};
use crate::app::{Launcher, NetForms, Role};
use crate::lobby::{GameState, start_enabled};
use crate::net::{LOBBY_MODE_CONTINUE, LobbyReady, NetState};

const COPIED_NOTICE_SECONDS: f64 = 2.0;

pub(super) fn session_open(forms: &NetForms) -> bool {
    !matches!(
        forms.net.status().state,
        NetState::Idle | NetState::Failed(_)
    )
}

pub(super) fn claim_mode(launcher: &mut Launcher, forms: &NetForms, mode: u8) -> bool {
    if !session_open(forms) {
        launcher.lobby_mode = mode;
        return true;
    }
    if launcher.lobby_mode == mode {
        return true;
    }
    false
}

pub(super) fn other_page_note(ui: &mut Ui, launcher: &Launcher) {
    ui.label(format!(
        "A lobby is open on the {} page.",
        ui_mode_name(launcher.lobby_mode)
    ));
}

pub(super) fn draw_lobby(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    let state = forms.net.status().state;
    match state {
        NetState::Connected { peer_name } => draw_connected(ui, launcher, forms, &peer_name),
        NetState::Hosting { ticket } => draw_hosting(ui, launcher, forms, &ticket),
        NetState::Joining => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("joining the host...");
            });
        }
        NetState::Idle | NetState::Failed(_) => draw_choose(ui, launcher, forms),
    }
    if let Some(error) = &launcher.play_error {
        ui.colored_label(BAD, error);
    }
}

fn draw_choose(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    let ctx = ui.ctx().clone();
    if crate::DEV_BUILD && launcher.game_running {
        ui.colored_label(
            WARN,
            "Dev build: Start runs a second game copy for the other player.",
        );
        if ui.button("Copy launch command").clicked() {
            launcher.copy_launch_command(&ctx);
        }
    }
    let host_block = launcher.lobby_block_reason(false);
    ui.horizontal(|ui| {
        let can_host = host_block.is_none() && !forms.hosting_pending();
        if ui
            .add_enabled(can_host, egui::Button::new("Host a lobby"))
            .clicked()
        {
            forms.start_host(&ctx);
        }
        if forms.hosting_pending() {
            ui.spinner();
            ui.label("starting...");
        }
    });
    if let Some(reason) = &host_block {
        ui.colored_label(WARN, reason);
    }
    if let Some(Err(error)) = &forms.host_result {
        ui.colored_label(BAD, error.to_string());
    }
    ui.separator();
    egui::Grid::new("join-form").num_columns(2).show(ui, |ui| {
        ui.label("Ticket");
        ui.add(egui::TextEdit::singleline(&mut forms.join_ticket).desired_width(f32::INFINITY));
        ui.end_row();
    });
    let has_ticket = !forms.join_ticket.trim().is_empty();
    let join_block = launcher.lobby_block_reason(true);
    ui.horizontal(|ui| {
        let can_join = join_block.is_none() && has_ticket && !forms.joining_pending();
        if ui
            .add_enabled(can_join, egui::Button::new("Join a lobby"))
            .clicked()
        {
            forms.start_join(&ctx);
        }
        if forms.joining_pending() {
            ui.spinner();
            ui.label("testing the connection...");
        }
    });
    if let Some(reason) = &join_block
        && host_block.as_ref() != Some(reason)
    {
        ui.colored_label(WARN, reason);
    }
    if let Some(Err(error)) = &forms.join_result {
        ui.colored_label(BAD, error.to_string());
    }
}

fn draw_hosting(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms, ticket: &str) {
    draw_ticket(ui, ticket);
    draw_player_list(ui, launcher, None);
    ui.separator();
    draw_host_options(ui, launcher, forms);
    ui.horizontal(|ui| {
        ui.spinner();
        ui.label("Waiting for a player to join...");
    });
    if ui.button("Close lobby").clicked() {
        forms.leave();
    }
}

fn draw_connected(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms, peer_name: &str) {
    let own = launcher.own_lobby_ready();
    let partner = launcher.link.peer_lobby;
    draw_player_list(ui, launcher, Some(peer_name));
    ui.separator();
    draw_host_options(ui, launcher, forms);
    if !own.ready
        && let Some(reason) = launcher.lobby_block_reason(launcher.link.guest)
    {
        ui.colored_label(WARN, reason);
    }
    let guest = launcher.link.guest;
    ui.horizontal(|ui| {
        if !guest {
            let enabled = start_enabled(true, false, launcher.lobby_started, own, partner);
            if ui
                .add_enabled(enabled, egui::Button::new("Start"))
                .clicked()
                && launcher.launch(Role::Host, own.mode)
            {
                forms.net.start_lobby(own.mode);
            }
        }
        if ui.button("Leave").clicked() {
            forms.leave();
            launcher.lobby_started = false;
        }
    });
}

pub(super) fn draw_game_state(ui: &mut Ui, state: GameState) {
    let text = format!("game: {}", state.text());
    match state {
        GameState::InPlay => ui.colored_label(GOOD, text),
        GameState::Closed => ui.colored_label(BAD, text),
        GameState::Starting | GameState::WaitingForPartner => ui.colored_label(WARN, text),
        GameState::NotStarted => ui.label(text),
    };
}

pub(super) fn draw_ready(ui: &mut Ui, ready: Option<LobbyReady>) {
    match ready {
        Some(ready) if ready.ready => {
            let what = match ready.slot {
                Some(slot) if ready.mode == LOBBY_MODE_CONTINUE => {
                    format!("ready (continue, slot {slot})")
                }
                _ => "ready (new game)".to_owned(),
            };
            ui.colored_label(GOOD, what);
        }
        Some(_) => {
            ui.colored_label(WARN, "not ready");
        }
        None => {
            ui.colored_label(WARN, "waiting");
        }
    }
}

fn draw_ticket(ui: &mut Ui, ticket: &str) {
    ui.label("Ticket (send this to your friend)");
    let mut shown = ticket;
    ui.add(egui::TextEdit::singleline(&mut shown).desired_width(f32::INFINITY));
    let copied_id = ui.make_persistent_id("ticket-copied-at");
    ui.horizontal(|ui| {
        if ui.button("Copy ticket").clicked() {
            ui.ctx().copy_text(ticket.to_owned());
            let now = ui.input(|input| input.time);
            ui.data_mut(|data| data.insert_temp(copied_id, now));
        }
        let copied_at: Option<f64> = ui.data(|data| data.get_temp(copied_id));
        if let Some(copied_at) = copied_at {
            let elapsed = ui.input(|input| input.time) - copied_at;
            if elapsed < COPIED_NOTICE_SECONDS {
                ui.colored_label(GOOD, "Copied");
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(250));
            }
        }
    });
}

fn ui_mode_name(mode: u8) -> &'static str {
    if mode == LOBBY_MODE_CONTINUE {
        "Continue"
    } else {
        "New game"
    }
}

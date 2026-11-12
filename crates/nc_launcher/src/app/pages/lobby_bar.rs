use eframe::egui::{
    Align, Align2, Layout, Rect, Response, Ui, UiBuilder, pos2, text::LayoutJob, vec2,
};
use nc_launcher_core::{StartInputs, start_allowed};

use crate::app::pages::bar_note::{BarNote, bar_note};
use crate::app::pages::ticket_field::ticket_field;
use crate::app::palette::{BAND, INK_MUTED, NOT_READY};
use crate::app::parts::{
    LINE_BUTTON_HEIGHT_PX, LineTone, caps_galley, command_button, copy_line, line_button,
    paint_double_rule,
};
use crate::app::style::{body_font, small_font};
use crate::app::{Launcher, NetForms, Role};
use crate::net::NetState;

const TICKET_LEFT_PX: f32 = 24.0;
const TICKET_TOP_PX: f32 = 24.0;
const TICKET_KEY_WIDTH_PX: f32 = 70.0;
const TICKET_KEY_TO_LINE_PX: f32 = 20.0;
const TICKET_LINE_WIDTH_PX: f32 = 470.0;
const TICKET_LINE_HEIGHT_PX: f32 = 33.0;
const ACTIONS_RIGHT_PX: f32 = 29.0;
const ACTIONS_TOP_PX: f32 = 18.0;
const ACTIONS_HEIGHT_PX: f32 = 52.0;
const ACTIONS_GAP_PX: f32 = 22.0;
const COMMAND_SIZE_PX: (f32, f32) = (196.0, 52.0);
const NOTE_GAP_PX: f32 = 24.0;
const NOTE_MIN_WIDTH_PX: f32 = 160.0;
const NOTE_ABOVE_BAR_PX: f32 = 22.0;
const TICKET_KEY: &str = "Ticket";
const HOST_LABEL: &str = "Host";
const JOIN_LABEL: &str = "Join";
const START_LABEL: &str = "Start";
const CLOSE_LOBBY_LABEL: &str = "Close lobby";
const LEAVE_LABEL: &str = "Leave";
const COPY_LAUNCH_COMMAND_LABEL: &str = "Copy launch command";
const RELAUNCH_LABEL: &str = "Relaunch and rejoin";
const TICKET_COPY_SALT: &str = "lobby-ticket";

enum TicketShown {
    Entry,
    Copy(String),
    Plain(String),
    Nothing,
}

pub(super) fn draw_lobby_bar(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    let bar = ui.max_rect();
    ui.painter().rect_filled(bar, 0.0, BAND);
    paint_double_rule(ui.painter(), bar.x_range(), bar.top());
    let state = forms.net.status().state;
    let note = bar_note(launcher, forms, &state);
    let actions_rect = Rect::from_min_max(
        pos2(bar.left(), bar.top() + ACTIONS_TOP_PX),
        pos2(
            bar.right() - ACTIONS_RIGHT_PX,
            bar.top() + ACTIONS_TOP_PX + ACTIONS_HEIGHT_PX,
        ),
    );
    let actions_left = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(actions_rect)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = ACTIONS_GAP_PX;
                draw_actions(ui, launcher, forms, &state)
            },
        )
        .inner;
    let key_left = bar.left() + TICKET_LEFT_PX;
    let line_left = key_left + TICKET_KEY_WIDTH_PX + TICKET_KEY_TO_LINE_PX;
    let line_width = TICKET_LINE_WIDTH_PX
        .min(actions_left - NOTE_GAP_PX - line_left)
        .max(0.0);
    let line_rect = Rect::from_min_size(
        pos2(line_left, bar.top() + TICKET_TOP_PX),
        vec2(line_width, TICKET_LINE_HEIGHT_PX),
    );
    let shown = ticket_shown(launcher, forms, &state);
    if !matches!(shown, TicketShown::Nothing) {
        let key = caps_galley(ui.painter(), TICKET_KEY, INK_MUTED);
        let key_top = line_rect.center().y - key.size().y / 2.0;
        ui.painter().galley(pos2(key_left, key_top), key, INK_MUTED);
    }
    match shown {
        TicketShown::Entry => ticket_field(ui, line_rect, &mut forms.join_ticket),
        TicketShown::Copy(ticket) => {
            ui.scope_builder(UiBuilder::new().max_rect(line_rect), |ui| {
                copy_line(
                    ui,
                    TICKET_COPY_SALT,
                    &ticket,
                    &ticket,
                    line_width,
                    body_font(),
                );
            });
        }
        TicketShown::Plain(text) => {
            ui.painter().text(
                pos2(line_rect.left(), line_rect.center().y),
                Align2::LEFT_CENTER,
                text,
                body_font(),
                INK_MUTED,
            );
        }
        TicketShown::Nothing => {}
    }
    if let Some(note) = note {
        let slot = Rect::from_min_max(
            pos2(line_rect.right() + NOTE_GAP_PX, actions_rect.top()),
            pos2(actions_left - NOTE_GAP_PX, actions_rect.bottom()),
        );
        paint_note(ui, slot, bar, &note);
    }
}

fn ticket_shown(launcher: &Launcher, forms: &NetForms, state: &NetState) -> TicketShown {
    match state {
        NetState::Idle | NetState::Failed(_) => TicketShown::Entry,
        NetState::Joining => TicketShown::Plain(forms.join_ticket.trim().to_owned()),
        NetState::Hosting { ticket } => TicketShown::Copy(ticket.clone()),
        NetState::Connected { .. } => match &launcher.link.ticket {
            Some(ticket) => TicketShown::Copy(ticket.clone()),
            None if !forms.join_ticket.trim().is_empty() => {
                TicketShown::Copy(forms.join_ticket.trim().to_owned())
            }
            None => TicketShown::Nothing,
        },
    }
}

fn draw_actions(
    ui: &mut Ui,
    launcher: &mut Launcher,
    forms: &mut NetForms,
    state: &NetState,
) -> f32 {
    let ctx = ui.ctx().clone();
    let last = match state {
        NetState::Idle | NetState::Failed(_) => {
            let can_host = launcher.lobby_block_reason(false).is_none() && !forms.hosting_pending();
            if command_button(ui, HOST_LABEL, COMMAND_SIZE_PX, true, can_host).clicked() {
                forms.start_host(&ctx);
            }
            let can_join = launcher.lobby_block_reason(true).is_none()
                && !forms.join_ticket.trim().is_empty()
                && !forms.joining_pending();
            let join = plain_button(ui, JOIN_LABEL, can_join);
            if join.clicked() {
                forms.start_join(&ctx);
            }
            if crate::DEV_BUILD && launcher.any_game_running {
                let copy = plain_button(ui, COPY_LAUNCH_COMMAND_LABEL, true);
                if copy.clicked() {
                    launcher.copy_launch_command(&ctx);
                }
                copy
            } else {
                join
            }
        }
        NetState::Joining | NetState::Hosting { .. } => {
            command_button(ui, START_LABEL, COMMAND_SIZE_PX, true, false);
            let label = if matches!(state, NetState::Joining) {
                LEAVE_LABEL
            } else {
                CLOSE_LOBBY_LABEL
            };
            let leave = plain_button(ui, label, true);
            if leave.clicked() {
                forms.leave();
                launcher.lobby_started = false;
            }
            leave
        }
        NetState::Connected { .. } => draw_connected_actions(ui, launcher, forms),
    };
    last.rect.left()
}

fn draw_connected_actions(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) -> Response {
    let guest = launcher.link.guest;
    let own = launcher.own_lobby_ready();
    let others: Vec<bool> = launcher
        .link
        .members
        .iter()
        .map(|member| member.lobby.is_some_and(|lobby| lobby.ready))
        .collect();
    let enabled = start_allowed(StartInputs {
        connected: true,
        viewer_is_guest: guest,
        already_started: launcher.lobby_started,
        own_ready: own.ready,
        others_ready: &others,
        rules: launcher.link.rules,
    });
    let start = command_button(ui, START_LABEL, COMMAND_SIZE_PX, true, enabled);
    if start.clicked() && launcher.launch(Role::Host, own.mode) {
        forms.net.start_lobby(own.mode);
    }
    let label = if guest {
        LEAVE_LABEL
    } else {
        CLOSE_LOBBY_LABEL
    };
    let leave = plain_button(ui, label, true);
    if leave.clicked() {
        forms.leave();
        launcher.lobby_started = false;
    }
    if launcher.follow_notice.is_some() && launcher.can_relaunch_and_rejoin() {
        let relaunch = plain_button(ui, RELAUNCH_LABEL, true);
        if relaunch.clicked() {
            launcher.relaunch_and_rejoin();
        }
        return relaunch;
    }
    leave
}

fn plain_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    line_button(
        ui,
        label,
        LineTone::Plain,
        LINE_BUTTON_HEIGHT_PX,
        None,
        enabled,
    )
}

fn paint_note(ui: &Ui, slot: Rect, bar: Rect, note: &BarNote) {
    let ink = if note.warm { NOT_READY } else { INK_MUTED };
    if slot.width() >= NOTE_MIN_WIDTH_PX {
        let mut job = LayoutJob::simple(note.text.clone(), small_font(), ink, slot.width());
        job.wrap.max_rows = 2;
        let galley = ui.painter().layout_job(job);
        let top = slot.center().y - galley.size().y / 2.0;
        ui.painter().galley(pos2(slot.left(), top), galley, ink);
        return;
    }
    // No room in the bar: the note sits just above it, at the right.
    ui.painter().text(
        pos2(
            bar.right() - ACTIONS_RIGHT_PX,
            bar.top() - NOTE_ABOVE_BAR_PX,
        ),
        Align2::RIGHT_CENTER,
        &note.text,
        small_font(),
        ink,
    );
}

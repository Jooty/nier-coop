use eframe::egui::{
    Align, Align2, Id, Layout, Pos2, Rect, Response, Ui, UiBuilder, pos2, text::LayoutJob, vec2,
};
use nc_launcher_core::{Easing, METER_FOLLOW_MS, StartInputs, start_allowed};

use crate::app::motion::ease_toward;
use crate::app::pages::bar_note::{BarNote, bar_note};
use crate::app::palette::{BAND, INK_MUTED, NOT_READY};
use crate::app::parts::{
    LINE_BUTTON_HEIGHT_PX, LineTone, METER_ROW_HEIGHT_PX, caps_galley, command_button, copy_line,
    line_button, meter_width, paint_double_rule, paint_meter_cells,
};
use crate::app::style::{body_font, small_font};
use crate::app::{Launcher, NetForms};
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
const NOTE_TO_METER_PX: f32 = 4.0;
const PROGRESS_CELLS: u32 = 12;
const PROGRESS_SALT: &str = "seat-send-progress";
const TICKET_KEY: &str = "Ticket";
const START_LABEL: &str = "Start";
const CLOSE_LOBBY_LABEL: &str = "Close lobby";
const LEAVE_LABEL: &str = "Leave";
const RELAUNCH_LABEL: &str = "Relaunch and rejoin";
const TICKET_COPY_SALT: &str = "lobby-ticket";

enum TicketShown {
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
        NetState::Idle | NetState::Failed(_) => TicketShown::Nothing,
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
    let last = match state {
        NetState::Idle | NetState::Failed(_) => return ui.max_rect().right(),
        NetState::Joining | NetState::Hosting { .. } => {
            command_button(ui, START_LABEL, COMMAND_SIZE_PX, true, false);
            let label = if matches!(state, NetState::Joining) {
                LEAVE_LABEL
            } else {
                CLOSE_LOBBY_LABEL
            };
            let leave = plain_button(ui, label, true);
            if leave.clicked() {
                launcher.before_leaving(&*forms.net);
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
    let enabled = !launcher.host_start_waiting()
        && start_allowed(StartInputs {
            connected: true,
            viewer_is_guest: guest,
            already_started: launcher.lobby_started,
            own_ready: own.ready,
            others_ready: &others,
            rules: launcher.link.rules,
        });
    let start = command_button(ui, START_LABEL, COMMAND_SIZE_PX, true, enabled);
    if start.clicked() {
        let now = ui.input(|input| input.time);
        launcher.press_start(&*forms.net, own.mode, now);
    }
    let label = if guest {
        LEAVE_LABEL
    } else {
        CLOSE_LOBBY_LABEL
    };
    let leave = plain_button(ui, label, true);
    if leave.clicked() {
        launcher.before_leaving(&*forms.net);
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

fn paint_progress(ui: &Ui, top_left: Pos2, (done, parts): (u16, u16)) {
    let share = f32::from(done) / f32::from(parts.max(1));
    // The cells follow the real progress a little behind, one after another, never ahead of it.
    let shown = ease_toward(
        ui,
        Id::unique(PROGRESS_SALT),
        share,
        METER_FOLLOW_MS,
        Easing::Linear,
    );
    let rect = Rect::from_min_size(
        top_left,
        vec2(meter_width(PROGRESS_CELLS), METER_ROW_HEIGHT_PX),
    );
    let filled = (shown * PROGRESS_CELLS as f32).floor() as u32;
    paint_meter_cells(ui.painter(), rect, PROGRESS_CELLS, filled, INK_MUTED);
}

fn paint_note(ui: &Ui, slot: Rect, bar: Rect, note: &BarNote) {
    let ink = if note.warm { NOT_READY } else { INK_MUTED };
    if slot.width() >= NOTE_MIN_WIDTH_PX {
        let mut job = LayoutJob::simple(note.text.clone(), small_font(), ink, slot.width());
        job.wrap.max_rows = 2;
        let galley = ui.painter().layout_job(job);
        let meter_height = note
            .progress
            .map_or(0.0, |_| NOTE_TO_METER_PX + METER_ROW_HEIGHT_PX);
        let top = slot.center().y - (galley.size().y + meter_height) / 2.0;
        let meter_top = top + galley.size().y + NOTE_TO_METER_PX;
        ui.painter().galley(pos2(slot.left(), top), galley, ink);
        if let Some(progress) = note.progress {
            paint_progress(ui, pos2(slot.left(), meter_top), progress);
        }
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

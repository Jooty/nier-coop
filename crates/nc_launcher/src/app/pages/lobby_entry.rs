use eframe::egui::{Align, Layout, Rect, Sense, Ui, UiBuilder, pos2, text::LayoutJob, vec2};

use crate::app::motion::page_section;
use crate::app::pages::bar_note::bar_note;
use crate::app::pages::big_name::{BIG_NAME_HEIGHT_PX, big_name_field};
use crate::app::pages::save_menu::draw_save_menu;
use crate::app::pages::ticket_field::ticket_field;
use crate::app::palette::{INK_MUTED, NOT_READY};
use crate::app::parts::{
    LINE_BUTTON_HEIGHT_PX, LineTone, MenuValue, command_button, line_button, menu_row,
    section_heading,
};
use crate::app::style::small_font;
use crate::app::{EntryChoice, Launcher, NetForms};

const ENTRY_MENU_WIDTH_PX: f32 = 288.0;
const ENTRY_MENU_TO_NEEDS_GAP_PX: f32 = 48.0;
const ENTRY_NEEDS_MAX_WIDTH_PX: f32 = 470.0;
const ENTRY_TOP_NUDGE_PX: f32 = 4.0;
const HEADING_TO_NAME_GAP_PX: f32 = 16.0;
const NAME_TO_CHOICES_GAP_PX: f32 = 34.0;
const HEADING_TO_ROWS_GAP_PX: f32 = 10.0;
const HEADING_TO_TICKET_GAP_PX: f32 = 12.0;
const TICKET_LINE_HEIGHT_PX: f32 = 33.0;
const NEEDS_TO_COMMAND_GAP_PX: f32 = 26.0;
const COMMAND_SIZE_PX: (f32, f32) = (196.0, 52.0);
const COMMAND_GAP_PX: f32 = 22.0;
const COMMAND_TO_NOTE_GAP_PX: f32 = 18.0;
const NOTE_MAX_ROWS: usize = 3;
const NAME_HEADING: &str = "Your name";
const LOBBY_HEADING: &str = "Lobby";
const HOST_A_LOBBY_LABEL: &str = "Host a lobby";
const JOIN_A_LOBBY_LABEL: &str = "Join a lobby";
const TICKET_HEADING: &str = "Ticket";
const HOST_LABEL: &str = "Host";
const JOIN_LABEL: &str = "Join";
const COPY_LAUNCH_COMMAND_LABEL: &str = "Copy launch command";

struct ChoiceMarks {
    heading_top: f32,
    rows_top: f32,
}

/// The New game or Continue tab before a lobby exists: your name above the two ways in, and beside the
/// pointed one what it needs (the host's saves on Continue, a ticket to join) with its command.
pub(super) fn draw_lobby_entry(
    ui: &mut Ui,
    launcher: &mut Launcher,
    forms: &mut NetForms,
    continuing: bool,
) {
    let page = ui.max_rect();
    let top = page.top() + ENTRY_TOP_NUDGE_PX;
    let menu_right = page.left() + ENTRY_MENU_WIDTH_PX;
    let menu = Rect::from_min_max(pos2(page.left(), top), pos2(menu_right, page.bottom()));
    let needs_left = menu_right + ENTRY_MENU_TO_NEEDS_GAP_PX;
    let needs_right = page.right().min(needs_left + ENTRY_NEEDS_MAX_WIDTH_PX);
    let marks = ui
        .scope_builder(UiBuilder::new().max_rect(menu), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            page_section(ui, 0, |ui| draw_entry_menu(ui, launcher, forms))
        })
        .inner;
    let host_without_saves = forms.entry_choice == EntryChoice::Host && !continuing;
    // The needs sit beside the choices; a lone command lines up with the rows, under no heading.
    let needs_top = if host_without_saves {
        marks.rows_top
    } else {
        marks.heading_top
    };
    let needs = Rect::from_min_max(
        pos2(needs_left, needs_top),
        pos2(needs_right, page.bottom()),
    );
    ui.scope_builder(UiBuilder::new().max_rect(needs), |ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        page_section(ui, 1, |ui| {
            match forms.entry_choice {
                EntryChoice::Host => draw_host_needs(ui, launcher, forms, continuing),
                EntryChoice::Join => draw_join_needs(ui, launcher, forms),
            }
            draw_entry_note(ui, launcher, forms);
        });
    });
}

fn draw_entry_menu(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) -> ChoiceMarks {
    section_heading(ui, NAME_HEADING, None);
    ui.add_space(HEADING_TO_NAME_GAP_PX);
    let (name_rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), BIG_NAME_HEIGHT_PX),
        Sense::hover(),
    );
    big_name_field(ui, name_rect, &mut launcher.new_game_name, true);
    ui.add_space(NAME_TO_CHOICES_GAP_PX);
    let heading_top = ui.cursor().top();
    section_heading(ui, LOBBY_HEADING, None);
    ui.add_space(HEADING_TO_ROWS_GAP_PX);
    let rows_top = ui.cursor().top();
    for (choice, label) in [
        (EntryChoice::Host, HOST_A_LOBBY_LABEL),
        (EntryChoice::Join, JOIN_A_LOBBY_LABEL),
    ] {
        let pointed = forms.entry_choice == choice;
        if menu_row(ui, label, MenuValue::None, pointed, true).clicked() {
            forms.entry_choice = choice;
        }
    }
    ChoiceMarks {
        heading_top,
        rows_top,
    }
}

fn draw_host_needs(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms, continuing: bool) {
    if continuing {
        draw_save_menu(ui, launcher);
        ui.add_space(NEEDS_TO_COMMAND_GAP_PX);
    }
    let can_host = launcher.lobby_block_reason(false).is_none() && !forms.hosting_pending();
    command_row(ui, launcher, |ui| {
        let host = command_button(ui, HOST_LABEL, COMMAND_SIZE_PX, true, can_host);
        if host.clicked() {
            forms.start_host(ui.ctx());
        }
    });
}

fn draw_join_needs(ui: &mut Ui, launcher: &mut Launcher, forms: &mut NetForms) {
    section_heading(ui, TICKET_HEADING, None);
    ui.add_space(HEADING_TO_TICKET_GAP_PX);
    let (line_rect, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), TICKET_LINE_HEIGHT_PX),
        Sense::hover(),
    );
    ticket_field(ui, line_rect, &mut forms.join_ticket);
    ui.add_space(NEEDS_TO_COMMAND_GAP_PX);
    let can_join = launcher.lobby_block_reason(true).is_none()
        && !forms.join_ticket.trim().is_empty()
        && !forms.joining_pending();
    command_row(ui, launcher, |ui| {
        let join = command_button(ui, JOIN_LABEL, COMMAND_SIZE_PX, true, can_join);
        if join.clicked() {
            forms.start_join(ui.ctx());
        }
    });
}

fn command_row(ui: &mut Ui, launcher: &mut Launcher, command: impl FnOnce(&mut Ui)) {
    let (row, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), COMMAND_SIZE_PX.1),
        Sense::hover(),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = COMMAND_GAP_PX;
            command(ui);
            if crate::DEV_BUILD && launcher.any_game_running {
                let copy = line_button(
                    ui,
                    COPY_LAUNCH_COMMAND_LABEL,
                    LineTone::Plain,
                    LINE_BUTTON_HEIGHT_PX,
                    None,
                    true,
                );
                if copy.clicked() {
                    launcher.copy_launch_command(ui.ctx());
                }
            }
        },
    );
}

fn draw_entry_note(ui: &mut Ui, launcher: &Launcher, forms: &NetForms) {
    let state = forms.net.status().state;
    let Some(note) = bar_note(launcher, forms, &state) else {
        return;
    };
    ui.add_space(COMMAND_TO_NOTE_GAP_PX);
    let ink = if note.warm { NOT_READY } else { INK_MUTED };
    let mut job = LayoutJob::simple(note.text, small_font(), ink, ui.available_width());
    job.wrap.max_rows = NOTE_MAX_ROWS;
    let galley = ui.painter().layout_job(job);
    let (rect, _) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, ink);
}

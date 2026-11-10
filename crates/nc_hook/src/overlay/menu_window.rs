use hudhook::imgui::{Condition, ItemHoveredFlags, TreeNodeFlags, Ui};
use nc_ui_style::{WARN, rgba_to_f32};

use crate::build_gate::{GateError, VerifiedBuild};
use crate::diagnostics::{log_line, log_path};
use crate::menu::{
    AreaEntry, CoOpStatus, ExtraPanel, FixEntry, FixOutcome, GameMenu, MenuNote, MenuSection,
    NoteTone, SectionBody, SectionKind, StoryJumpEntry, build_menu,
};
use crate::overlay::OverlayGame;
use crate::overlay::menu_state::MenuState;

const AREA_LIST_HEIGHT: f32 = 200.0;

pub(super) fn watch_area_changes(game: &dyn GameMenu, state: &mut MenuState) {
    if !state.area_changes.is_waiting() {
        return;
    }
    let now = state.now_ms();
    if let Some(outcome) = state
        .area_changes
        .observe(game.current_area().as_ref(), now)
    {
        log_line(&outcome.log_line());
    }
}

pub(super) fn draw_menu_window<G: OverlayGame>(
    ui: &Ui,
    game: &mut G,
    gate: &Result<VerifiedBuild, GateError>,
    state: &mut MenuState,
) {
    let model = build_menu(&*game);
    ui.window("menu##debug")
        .position([16.0, 16.0], Condition::FirstUseEver)
        .size([340.0, 460.0], Condition::FirstUseEver)
        .build(|| {
            for section in &model.sections {
                let flags = if section.kind == SectionKind::CoOp {
                    TreeNodeFlags::DEFAULT_OPEN
                } else {
                    TreeNodeFlags::empty()
                };
                if !ui.collapsing_header(section.kind.title(), flags) {
                    continue;
                }
                draw_section(ui, game, gate, state, section);
                draw_notes(ui, &section.notes);
                if let Some(note) = &section.empty_note {
                    ui.text_disabled(note);
                }
            }
        });
}

pub(super) fn draw_extra_panels<G: OverlayGame>(ui: &Ui, game: &mut G, state: &mut MenuState) {
    for panel in game.extra_panels() {
        let open = state.open_panels.entry(panel.id).or_insert(false);
        if *open {
            game.draw_extra_panel(ui, panel.id, open);
        }
    }
}

fn draw_section<G: OverlayGame>(
    ui: &Ui,
    game: &mut G,
    gate: &Result<VerifiedBuild, GateError>,
    state: &mut MenuState,
    section: &MenuSection,
) {
    match &section.body {
        SectionBody::CoOp(status) => draw_co_op(ui, status),
        SectionBody::Teleport { current, areas } => {
            draw_teleport(ui, game, state, current.as_ref(), areas);
        }
        SectionBody::StoryJump(jumps) => draw_story_jumps(ui, game, state, jumps),
        SectionBody::Fixes(fixes) => draw_fixes(ui, game, state, fixes),
        SectionBody::SettingsAndLogs {
            status_lines,
            panels,
        } => {
            game.draw_settings(ui);
            ui.separator();
            if let Err(error) = gate {
                ui.text(format!("build gate: FAIL ({error})"));
            }
            for line in status_lines {
                ui.text(line);
            }
            draw_panel_buttons(ui, state, panels);
            ui.text_wrapped(format!("log file: {}", log_path()));
        }
    }
}

fn draw_co_op(ui: &Ui, status: &CoOpStatus) {
    ui.text(format!("link: {}", status.link));
    for player in &status.players {
        ui.text(format!("{}: {}", player.name, player.detail));
    }
    match status.ping_ms {
        Some(ping) => ui.text(format!("ping: {ping} ms")),
        None => ui.text("ping: unknown"),
    }
}

fn draw_teleport<G: OverlayGame>(
    ui: &Ui,
    game: &G,
    state: &mut MenuState,
    current: Option<&AreaEntry>,
    areas: &[AreaEntry],
) {
    if ui.button("Go to partner") {
        state.teleport_message = describe(game.go_to_partner());
    }
    ui.same_line();
    if ui.button("Bring partner here") {
        state.teleport_message = describe(game.bring_partner());
    }
    if !state.teleport_message.is_empty() {
        ui.text_wrapped(&state.teleport_message);
    }
    ui.separator();
    ui.text("map teleporter");
    ui.text(format!(
        "current: {}",
        current.map_or("unknown", |area| area.name.as_str())
    ));
    if areas.is_empty() {
        return;
    }
    ui.input_text("filter", &mut state.area_filter).build();
    let filter = state.area_filter.to_ascii_lowercase();
    let mut chosen: Option<&AreaEntry> = None;
    ui.child_window("area list")
        .size([0.0, AREA_LIST_HEIGHT])
        .border(true)
        .build(|| {
            for area in areas
                .iter()
                .filter(|area| area.name.to_ascii_lowercase().contains(&filter))
            {
                let here = current.is_some_and(|current| current.id == area.id);
                if ui.selectable_config(&area.name).selected(here).build() {
                    chosen = Some(area);
                }
            }
        });
    if let Some(target) = chosen {
        request_area_change(game, state, current, target);
    }
    let message = state.area_changes.message();
    if !message.is_empty() {
        ui.text_wrapped(message);
    }
}

fn request_area_change<G: GameMenu>(
    game: &G,
    state: &mut MenuState,
    current: Option<&AreaEntry>,
    target: &AreaEntry,
) {
    log_line(&format!(
        "teleport: asked to change the area to {}",
        target.name
    ));
    let now = state.now_ms();
    match state
        .area_changes
        .request(target, current, game.area_change_blocker(), now)
    {
        Err(refusal) => log_line(&format!("teleport: refused ({refusal})")),
        Ok(()) => {
            if let Err(reason) = game.change_area(target) {
                state.area_changes.game_refused(&reason);
                log_line(&format!("teleport: refused ({reason})"));
            }
        }
    }
}

fn draw_story_jumps<G: OverlayGame>(
    ui: &Ui,
    game: &G,
    state: &mut MenuState,
    jumps: &[StoryJumpEntry],
) {
    for jump in jumps {
        if ui.button(&jump.label) {
            state.story_jump_message = describe(game.run_story_jump(jump.id));
        }
    }
    if !state.story_jump_message.is_empty() {
        ui.text_wrapped(&state.story_jump_message);
    }
}

fn draw_fixes<G: OverlayGame>(ui: &Ui, game: &G, state: &mut MenuState, fixes: &[FixEntry]) {
    for fix in fixes {
        let pressed = {
            let _disabled = ui.begin_disabled(fix.disabled_reason.is_some());
            ui.button(&fix.label)
        };
        if let Some(reason) = &fix.disabled_reason
            && ui.is_item_hovered_with_flags(ItemHoveredFlags::ALLOW_WHEN_DISABLED)
        {
            ui.tooltip_text(reason);
        }
        if pressed {
            state.fix_message = match game.run_fix(fix.id) {
                FixOutcome::Done => String::new(),
                FixOutcome::CopyToClipboard(text) => {
                    ui.set_clipboard_text(text);
                    String::from("copied to the clipboard")
                }
                FixOutcome::Refused(reason) => reason,
            };
        }
    }
    if !state.fix_message.is_empty() {
        ui.text_wrapped(&state.fix_message);
    }
}

fn draw_panel_buttons(ui: &Ui, state: &mut MenuState, panels: &[ExtraPanel]) {
    for (index, panel) in panels.iter().enumerate() {
        if index > 0 {
            ui.same_line();
        }
        if ui.button(panel.label) {
            let open = state.open_panels.entry(panel.id).or_insert(false);
            *open = !*open;
        }
    }
}

fn draw_notes(ui: &Ui, notes: &[MenuNote]) {
    for note in notes {
        match note.tone {
            NoteTone::Plain => ui.text_wrapped(&note.text),
            NoteTone::Warn => ui.text_colored(rgba_to_f32(WARN), &note.text),
        }
    }
}

fn describe(result: Result<String, String>) -> String {
    match result {
        Ok(text) | Err(text) => text,
    }
}

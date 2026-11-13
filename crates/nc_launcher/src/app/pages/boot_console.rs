use eframe::egui::{
    self, Align, Event, FontId, Frame, Id, Key, Layout, Rect, Sense, Ui, UiBuilder, pos2, vec2,
};
use nc_launcher_core::{CONSOLE_LEADER, ConsoleTask, ConsoleView};

use crate::app::pages::lobby::session_open;
use crate::app::pages::tabs::lobby_tab;
use crate::app::palette::{CONSOLE_GROUND, INK, INK_MUTED, INK_UNAVAILABLE, NOT_READY, READY};
use crate::app::parts::{LINE_BUTTON_HEIGHT_PX, LineTone, line_button};
use crate::app::{JoinDialog, Launcher, NetForms, Screen, Tab, console_script};

const CONSOLE_FONT_PX: f32 = 15.0;
const CONSOLE_LINE_HEIGHT_PX: f32 = 26.0;
const CONSOLE_LEFT_SHARE: f32 = 0.12;
const CONSOLE_TOP_SHARE: f32 = 0.28;
const CONSOLE_MIN_LEFT_PX: f32 = 48.0;
const CONSOLE_WORD_GAP_PX: f32 = 12.0;
const CONSOLE_CURSOR_GAP_PX: f32 = 2.0;
const CONSOLE_ERROR_WIDTH_PX: f32 = 560.0;
const CONSOLE_ERROR_TO_BACK_PX: f32 = 22.0;
const CONSOLE_CLICK_SALT: &str = "boot-console-click";
const BACK_LABEL: &str = "Back";

pub(super) fn draw_boot_console(
    ui: &mut Ui,
    tab: &mut Tab,
    launcher: &mut Launcher,
    forms: &mut NetForms,
) {
    let Some(task) = forms.console.as_ref().map(|run| run.task) else {
        return;
    };
    let script = console_script(launcher, forms, task);
    let now_ms = ui.input(|input| input.time) * 1000.0;
    let motion = launcher.config.motion;
    let Some(run) = forms.console.as_mut() else {
        return;
    };
    run.clock.observe(&script, now_ms);
    let mut view = run.clock.view(&script, now_ms, motion);
    let (escape, other_key) = ui.input(|input| {
        let escape = input.key_pressed(Key::Escape);
        let other = input.events.iter().any(|event| {
            matches!(event, Event::Key { key, pressed: true, .. } if *key != Key::Escape)
        });
        (escape, other)
    });
    let mut back = false;
    egui::CentralPanel::default()
        .frame(Frame::NONE.fill(CONSOLE_GROUND))
        .show(ui, |ui| {
            let whole = ui.max_rect();
            let clicked = ui
                .interact(whole, Id::unique(CONSOLE_CLICK_SALT), Sense::click())
                .clicked();
            if view.failed {
                back = escape || clicked;
            } else if escape {
                back = true;
            } else if clicked || other_key {
                run.clock.skip();
                view = run.clock.view(&script, now_ms, motion);
            }
            if paint_console(ui, whole, &view) {
                back = true;
            }
        });
    if back {
        forms.abandon();
        if task == ConsoleTask::Joining {
            forms.join_dialog = Some(JoinDialog::default());
        }
        return;
    }
    if view.lobby_due && session_open(forms) {
        forms.console = None;
        launcher.screen = Screen::Game;
        *tab = lobby_tab(launcher.lobby_mode);
        ui.ctx().request_repaint();
        return;
    }
    if view.moving || view.lobby_due {
        ui.ctx().request_repaint();
    }
}

fn paint_console(ui: &mut Ui, whole: Rect, view: &ConsoleView) -> bool {
    let left = whole.left() + (whole.width() * CONSOLE_LEFT_SHARE).max(CONSOLE_MIN_LEFT_PX);
    let mut top = whole.top() + whole.height() * CONSOLE_TOP_SHARE;
    let font = FontId::monospace(CONSOLE_FONT_PX);
    let painter = ui.painter().clone();
    for (index, line) in view.lines.iter().enumerate() {
        let middle = top + CONSOLE_LINE_HEIGHT_PX / 2.0;
        let label_end = line.typed.find(CONSOLE_LEADER).unwrap_or(line.typed.len());
        let (label, leaders) = line.typed.split_at(label_end);
        let label_galley = painter.layout_no_wrap(label.to_owned(), font.clone(), INK);
        let leaders_left = left + label_galley.size().x;
        let text_top = middle - label_galley.size().y / 2.0;
        painter.galley(pos2(left, text_top), label_galley, INK);
        let leaders_galley =
            painter.layout_no_wrap(leaders.to_owned(), font.clone(), INK_UNAVAILABLE);
        let typed_right = leaders_left + leaders_galley.size().x;
        painter.galley(pos2(leaders_left, text_top), leaders_galley, INK_UNAVAILABLE);
        if let Some(word) = line.word {
            let ink = if line.failed { NOT_READY } else { READY };
            let word_galley = painter.layout_no_wrap(word.to_owned(), font.clone(), ink);
            painter.galley(
                pos2(typed_right + CONSOLE_WORD_GAP_PX, text_top),
                word_galley,
                ink,
            );
        }
        if view.cursor_line == Some(index) {
            let glyph = painter.layout_no_wrap("M".to_owned(), font.clone(), INK);
            let cursor = Rect::from_min_size(
                pos2(typed_right + CONSOLE_CURSOR_GAP_PX, text_top),
                glyph.size(),
            );
            painter.rect_filled(cursor, 0.0, INK);
        }
        top += CONSOLE_LINE_HEIGHT_PX;
    }
    if !view.failed {
        return false;
    }
    if let Some(error) = &view.error {
        let width = CONSOLE_ERROR_WIDTH_PX.min(whole.right() - left - CONSOLE_MIN_LEFT_PX);
        let galley = painter.layout(error.clone(), font, INK_MUTED, width.max(0.0));
        let height = galley.size().y;
        painter.galley(
            pos2(left, top + (CONSOLE_LINE_HEIGHT_PX - CONSOLE_FONT_PX) / 2.0),
            galley,
            INK_MUTED,
        );
        top += height.max(CONSOLE_LINE_HEIGHT_PX);
    }
    top += CONSOLE_ERROR_TO_BACK_PX;
    let row = Rect::from_min_size(
        pos2(left, top),
        vec2(whole.right() - left, LINE_BUTTON_HEIGHT_PX),
    );
    ui.scope_builder(
        UiBuilder::new()
            .max_rect(row)
            .layout(Layout::left_to_right(Align::Center)),
        |ui| {
            line_button(
                ui,
                BACK_LABEL,
                LineTone::Plain,
                LINE_BUTTON_HEIGHT_PX,
                None,
                true,
            )
            .clicked()
        },
    )
    .inner
}

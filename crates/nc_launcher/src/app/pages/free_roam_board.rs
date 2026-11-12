use eframe::egui::{Rangef, Rect, Ui, pos2};
use nc_launcher_core::{ROSTER_COLUMN_SEATS, RosterSeat, remove_offered, roster, strip_art};

use crate::app::pages::board_strip::{BOARD_STRIP_HEIGHT_PX, draw_board_strip};
use crate::app::pages::own_pick::{OwnPick, apply_pick, body_pick, character_pick};
use crate::app::pages::players_heading::{PLAYERS_HEADING_HEIGHT_PX, players_heading};
use crate::app::pages::roster_grid::{RosterGrid, roster_grid};
use crate::app::pages::roster_row::{
    ROSTER_HEADER_HEIGHT_PX, ROSTER_ROW_HEIGHT_PX, RosterLine, RowLook, draw_roster_row,
    paint_open_row, paint_roster_header,
};
use crate::app::pages::seat::SeatName;
use crate::app::pages::seat_ledger::PingCell;
use crate::app::{Launcher, NetForms};
use crate::home::BannerArtState;
use crate::lobby::{
    PlayerRole, PlayerStatus, body_label, character_id_at, character_label_of,
    lobby_picks_character_id, player_status,
};
use crate::net::MemberStatus;

const HEADING_TO_STRIP_GAP_PX: f32 = 14.0;
const STRIP_TO_BOARD_GAP_PX: f32 = 16.0;
const ROSTER_COLUMN_GAP_PX: f32 = 16.0;
const UNSEEN_CHARACTER_TEXT: &str = "-";

/// The free roam lobby's right side: the art strip with your character, and the roster of every seat.
pub(super) fn draw_free_roam_board(
    ui: &mut Ui,
    launcher: &mut Launcher,
    forms: &NetForms,
    name_editable: bool,
) {
    let main = ui.max_rect();
    let viewer_is_guest = launcher.link.guest;
    let party = 1 + launcher.link.members.len();
    players_heading(ui, party, launcher.link.rules.max_players);
    let strip = Rect::from_min_max(
        pos2(
            main.left(),
            main.top() + PLAYERS_HEADING_HEIGHT_PX + HEADING_TO_STRIP_GAP_PX,
        ),
        pos2(
            main.right(),
            main.top()
                + PLAYERS_HEADING_HEIGHT_PX
                + HEADING_TO_STRIP_GAP_PX
                + BOARD_STRIP_HEIGHT_PX,
        ),
    );
    let art = match launcher
        .banner_art
        .state(ui.ctx(), launcher.profile.game_id)
    {
        BannerArtState::Ready(art) => Some(art.colour),
        BannerArtState::Pending | BannerArtState::Missing => None,
    };
    let pick_now = own_pick(launcher, viewer_is_guest);
    let crop = strip_art(launcher.profile.game_id);
    if let Some(step) = draw_board_strip(ui, strip, art.as_ref(), crop, &pick_now)
        && let Some(pick) = pick_now.step(step)
    {
        apply_pick(launcher, pick);
    }
    let own_label = own_pick(launcher, viewer_is_guest).label;
    let board_top = strip.bottom() + STRIP_TO_BOARD_GAP_PX;
    let slots: Vec<u8> = launcher
        .link
        .members
        .iter()
        .map(|member| member.slot)
        .collect();
    let board = roster(&slots, launcher.link.own_slot.unwrap_or(0));
    let column_width =
        (main.width() - ROSTER_COLUMN_GAP_PX * (board.columns - 1) as f32) / board.columns as f32;
    let grids: Vec<(Rangef, RosterGrid)> = (0..board.columns)
        .map(|column| {
            let left = main.left() + column as f32 * (column_width + ROSTER_COLUMN_GAP_PX);
            let range = Rangef::new(left, left + column_width);
            (range, roster_grid(range, board.columns == 1))
        })
        .collect();
    for (range, grid) in &grids {
        let header = Rect::from_x_y_ranges(
            *range,
            Rangef::new(board_top, board_top + ROSTER_HEADER_HEIGHT_PX),
        );
        paint_roster_header(ui, header, grid);
    }
    let row_rect = |index: usize| {
        let (range, _) = &grids[index / ROSTER_COLUMN_SEATS];
        let top = board_top
            + ROSTER_HEADER_HEIGHT_PX
            + (index % ROSTER_COLUMN_SEATS) as f32 * ROSTER_ROW_HEIGHT_PX;
        Rect::from_x_y_ranges(*range, Rangef::new(top, top + ROSTER_ROW_HEIGHT_PX))
    };
    let cursor = board.seats.iter().enumerate().position(|(index, seat)| {
        *seat != RosterSeat::Open && ui.rect_contains_pointer(row_rect(index))
    });
    let mut order: Vec<usize> = (0..board.seats.len())
        .filter(|index| Some(*index) != cursor)
        .collect();
    order.extend(cursor);
    let mut removed = None;
    for index in order {
        let rect = row_rect(index);
        let grid = &grids[index / ROSTER_COLUMN_SEATS].1;
        let last = index % ROSTER_COLUMN_SEATS == ROSTER_COLUMN_SEATS - 1;
        let look = if Some(index) == cursor {
            RowLook::Cursor
        } else {
            RowLook::Rest
        };
        match board.seats[index] {
            RosterSeat::Open => paint_open_row(ui, rect, grid, index + 1, last),
            RosterSeat::Own => {
                let status = launcher.your_player_status();
                let line = RosterLine {
                    name: SeatName::Own {
                        text: &mut launcher.new_game_name,
                        editable: name_editable,
                    },
                    you: true,
                    role: own_role(viewer_is_guest).text(),
                    character: own_label.to_owned(),
                    status,
                    ping: PingCell::Local,
                    removable: false,
                };
                draw_roster_row(ui, rect, grid, index + 1, line, look, last);
            }
            RosterSeat::Member(at) => {
                let member = &launcher.link.members[at];
                let line = RosterLine {
                    name: SeatName::Fixed(&member.name),
                    you: false,
                    role: member_role(member).text(),
                    character: member_character(launcher, member).to_owned(),
                    status: member_status(member),
                    ping: member.rtt_ms.map_or(PingCell::Unknown, PingCell::Millis),
                    removable: remove_offered(viewer_is_guest, false),
                };
                if draw_roster_row(ui, rect, grid, index + 1, line, look, last) {
                    removed = Some(member.slot);
                }
            }
        }
    }
    if let Some(slot) = removed {
        forms.net.remove_member(slot);
    }
}

fn own_pick(launcher: &Launcher, viewer_is_guest: bool) -> OwnPick {
    if lobby_picks_character_id(launcher.profile) {
        return character_pick(launcher, viewer_is_guest);
    }
    let body = launcher.own_lobby_body();
    if viewer_is_guest {
        return body_pick(launcher.profile, body);
    }
    OwnPick {
        label: body_label(body),
        choices: Vec::new(),
    }
}

fn own_role(viewer_is_guest: bool) -> PlayerRole {
    if viewer_is_guest {
        PlayerRole::Guest
    } else {
        PlayerRole::Host
    }
}

fn member_role(member: &MemberStatus) -> PlayerRole {
    if member.slot == 0 {
        PlayerRole::Host
    } else {
        PlayerRole::Guest
    }
}

fn member_character(launcher: &Launcher, member: &MemberStatus) -> &'static str {
    let profile = launcher.profile;
    if lobby_picks_character_id(profile) {
        return member
            .character
            .and_then(|index| character_id_at(profile, index))
            .map_or(UNSEEN_CHARACTER_TEXT, |id| character_label_of(profile, id));
    }
    member.body.map_or(UNSEEN_CHARACTER_TEXT, body_label)
}

fn member_status(member: &MemberStatus) -> PlayerStatus {
    let ready = member.lobby.is_some_and(|lobby| lobby.ready);
    player_status(ready, member.readiness, false)
}

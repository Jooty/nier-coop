use eframe::egui::{Align2, FontId, Rect, Ui, pos2};
use nc_launcher_core::seat_art;
use replicant_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::pages::seat::{Seat, SeatName, draw_seat};
use crate::app::pages::seat_ledger::{CharacterCell, PingCell, SeatLedger, Step};
use crate::app::palette::{INK, INK_MUTED};
use crate::app::parts::section_heading;
use crate::app::style::light_family;
use crate::home::BannerArtState;
use crate::lobby::{
    PlayerRole, PlayerRow, RowInputs, body_label, character_label_of, lobby_picks_character_id,
    picker_choices, player_rows,
};

const PLAYERS_HEADING: &str = "Players";
const PARTNER_CHARACTER_UNSEEN_TEXT: &str = "chosen on their side";
const OPEN_SEAT_NAME: &str = "Waiting";
const HEADING_HEIGHT_PX: f32 = 32.0;
const HEADING_TO_SEATS_GAP_PX: f32 = 14.0;
const SEAT_GAP_PX: f32 = 28.0;
const COUNT_SIZE_PX: f32 = 22.0;

/// What the seats need from the session: who the partner is, and whether your name may still change.
pub(super) struct SeatFacts<'a> {
    pub peer_name: Option<&'a str>,
    pub name_editable: bool,
}

enum Pick {
    Character(&'static str),
    Body(PartnerBody),
}

pub(super) fn draw_player_seats(ui: &mut Ui, launcher: &mut Launcher, facts: SeatFacts) {
    let main = ui.max_rect();
    let viewer_is_guest = launcher.link.guest && facts.peer_name.is_some();
    let party = 1 + usize::from(facts.peer_name.is_some());
    paint_heading(ui, main, party, launcher.link.rules.max_players);
    let seats_top = main.top() + HEADING_HEIGHT_PX + HEADING_TO_SEATS_GAP_PX;
    let seat_width = ((main.width() - SEAT_GAP_PX) / 2.0).max(0.0);
    let own_name = launcher.effective_player_name();
    let guest_body = guest_body_shown(launcher, viewer_is_guest);
    let rows = player_rows(RowInputs {
        viewer_is_guest,
        own_name: &own_name,
        peer_name: facts.peer_name,
        guest_body,
    });
    let art = match launcher
        .banner_art
        .state(ui.ctx(), launcher.profile.game_id)
    {
        BannerArtState::Ready(art) => Some(art.colour),
        BannerArtState::Pending | BannerArtState::Missing => None,
    };
    let mut pick = None;
    for (index, role) in [PlayerRole::Host, PlayerRole::Guest]
        .into_iter()
        .enumerate()
    {
        let rect = Rect::from_min_max(
            pos2(
                main.left() + index as f32 * (seat_width + SEAT_GAP_PX),
                seats_top,
            ),
            pos2(
                main.left() + index as f32 * (seat_width + SEAT_GAP_PX) + seat_width,
                main.bottom(),
            ),
        );
        let row = rows.iter().find(|row| row.role == role);
        let (ledger, choices) = match row {
            Some(row) => ledger_for(launcher, row, viewer_is_guest),
            None => (open_ledger(), Vec::new()),
        };
        let name = match row {
            Some(row) if row.you => SeatName::Own {
                text: &mut launcher.new_game_name,
                editable: facts.name_editable,
            },
            Some(row) => SeatName::Fixed(&row.name),
            None => SeatName::Open(OPEN_SEAT_NAME),
        };
        let seat = Seat {
            number: index + 1,
            role: role.text(),
            you: row.is_some_and(|row| row.you),
            name,
            ledger,
        };
        let crop = seat_art(launcher.profile.game_id, index);
        if let Some(step) = draw_seat(ui, rect, seat, art.as_ref(), crop) {
            pick = step_choice(&choices, step);
        }
    }
    match pick {
        Some(Pick::Character(id)) => launcher.set_character_id(id),
        Some(Pick::Body(body)) => launcher.set_body(body),
        None => {}
    }
}

fn paint_heading(ui: &mut Ui, main: Rect, party: usize, max_players: u8) {
    section_heading(ui, PLAYERS_HEADING, None);
    let painter = ui.painter();
    let font = FontId::new(COUNT_SIZE_PX, light_family());
    let middle = pos2(main.right(), main.top() + HEADING_HEIGHT_PX / 2.0 - 4.0);
    let of = painter.text(
        middle,
        Align2::RIGHT_CENTER,
        format!(" / {max_players}"),
        font.clone(),
        INK_MUTED,
    );
    painter.text(
        pos2(of.left(), middle.y),
        Align2::RIGHT_CENTER,
        party.to_string(),
        font,
        INK,
    );
}

fn ledger_for(
    launcher: &mut Launcher,
    row: &PlayerRow,
    viewer_is_guest: bool,
) -> (SeatLedger, Vec<Pick>) {
    let profile = launcher.profile;
    let (character, choices) = if lobby_picks_character_id(profile) {
        if row.you {
            let own = launcher.own_character_id(viewer_is_guest);
            let choices: Vec<Pick> = launcher
                .own_character_choices(viewer_is_guest)
                .into_iter()
                .map(|choice| Pick::Character(choice.id))
                .collect();
            let ordered = rotate_to(
                choices,
                |pick| matches!(pick, Pick::Character(id) if *id == own),
            );
            (
                stepper_or_fixed(character_label_of(profile, own), &ordered),
                ordered,
            )
        } else {
            let label = launcher
                .partner_character_id(viewer_is_guest)
                .map_or(PARTNER_CHARACTER_UNSEEN_TEXT, |id| {
                    character_label_of(profile, id)
                });
            (CharacterCell::Fixed(label.to_owned()), Vec::new())
        }
    } else if row.picker {
        let choices: Vec<Pick> = picker_choices(profile, &[PartnerBody::Nier])
            .into_iter()
            .map(Pick::Body)
            .collect();
        let ordered = rotate_to(
            choices,
            |pick| matches!(pick, Pick::Body(body) if *body == row.body),
        );
        (stepper_or_fixed(body_label(row.body), &ordered), ordered)
    } else {
        (
            CharacterCell::Fixed(body_label(row.body).to_owned()),
            Vec::new(),
        )
    };
    let status = if row.you {
        launcher.your_player_status()
    } else {
        launcher.their_player_status()
    };
    let ping = if row.you {
        PingCell::Local
    } else {
        launcher
            .link
            .partner()
            .and_then(|partner| partner.rtt_ms)
            .or_else(|| {
                launcher
                    .link
                    .rtt
                    .map(|rtt| u32::try_from(rtt.as_millis()).unwrap_or(u32::MAX))
            })
            .map_or(PingCell::Unknown, PingCell::Millis)
    };
    (
        SeatLedger {
            character,
            status: Some(status),
            ping,
        },
        choices,
    )
}

fn open_ledger() -> SeatLedger {
    SeatLedger {
        character: CharacterCell::Empty,
        status: None,
        ping: PingCell::Unknown,
    }
}

fn stepper_or_fixed(label: &'static str, choices: &[Pick]) -> CharacterCell {
    if choices.len() > 1 {
        CharacterCell::Stepper(label)
    } else {
        CharacterCell::Fixed(label.to_owned())
    }
}

fn rotate_to(mut choices: Vec<Pick>, is_current: impl Fn(&Pick) -> bool) -> Vec<Pick> {
    // The current pick goes first, so a step lands on its neighbour at either end.
    if let Some(at) = choices.iter().position(is_current) {
        choices.rotate_left(at);
    }
    choices
}

fn step_choice(choices: &[Pick], step: Step) -> Option<Pick> {
    let next = match step {
        Step::Forward => choices.get(1),
        Step::Back => choices.last().filter(|_| choices.len() > 1),
    }?;
    Some(match next {
        Pick::Character(id) => Pick::Character(id),
        Pick::Body(body) => Pick::Body(*body),
    })
}

fn guest_body_shown(launcher: &mut Launcher, viewer_is_guest: bool) -> PartnerBody {
    if lobby_picks_character_id(launcher.profile) {
        return PartnerBody::Nier;
    }
    if !viewer_is_guest {
        return launcher
            .link
            .partner()
            .and_then(|partner| partner.body)
            .unwrap_or_default();
    }
    let own = launcher.own_lobby_body();
    if !picker_choices(launcher.profile, &[PartnerBody::Nier]).contains(&own) {
        launcher.set_body(PartnerBody::Nier);
        return PartnerBody::Nier;
    }
    own
}

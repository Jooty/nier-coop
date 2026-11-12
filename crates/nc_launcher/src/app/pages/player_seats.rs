use eframe::egui::{Rect, Ui, pos2};
use nc_launcher_core::seat_art;
use replicant_proto::PartnerBody;

use crate::app::Launcher;
use crate::app::pages::own_pick::{OwnPick, apply_pick, body_pick, character_pick};
use crate::app::pages::players_heading::{PLAYERS_HEADING_HEIGHT_PX, players_heading};
use crate::app::pages::seat::{Seat, SeatName, draw_seat};
use crate::app::pages::seat_ledger::{CharacterCell, PingCell, SeatLedger};
use crate::home::BannerArtState;
use crate::lobby::{
    PlayerRole, PlayerRow, RowInputs, body_label, character_label_of, lobby_picks_character_id,
    picker_choices, player_rows,
};

const PARTNER_CHARACTER_UNSEEN_TEXT: &str = "Their choice";
const HEADING_TO_SEATS_GAP_PX: f32 = 14.0;
const SEAT_GAP_PX: f32 = 28.0;

/// What the seats need from the session: who the partner is, and whether your name may still change.
pub(super) struct SeatFacts<'a> {
    pub peer_name: Option<&'a str>,
    pub name_editable: bool,
    pub open_seat_name: &'static str,
}

pub(super) fn draw_player_seats(ui: &mut Ui, launcher: &mut Launcher, facts: SeatFacts) {
    let main = ui.max_rect();
    let viewer_is_guest = launcher.link.guest && facts.peer_name.is_some();
    let party = 1 + launcher.link.members.len();
    players_heading(ui, party, launcher.link.rules.max_players);
    let seats_top = main.top() + PLAYERS_HEADING_HEIGHT_PX + HEADING_TO_SEATS_GAP_PX;
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
        let (ledger, own_pick) = match row {
            Some(row) => ledger_for(launcher, row, viewer_is_guest),
            None => (open_ledger(), None),
        };
        let name = match row {
            Some(row) if row.you => SeatName::Own {
                text: &mut launcher.new_game_name,
                editable: facts.name_editable,
            },
            Some(row) => SeatName::Fixed(&row.name),
            None => SeatName::Open(facts.open_seat_name),
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
            pick = own_pick.and_then(|own: OwnPick| own.step(step));
        }
    }
    if let Some(pick) = pick {
        apply_pick(launcher, pick);
    }
}

fn ledger_for(
    launcher: &mut Launcher,
    row: &PlayerRow,
    viewer_is_guest: bool,
) -> (SeatLedger, Option<OwnPick>) {
    let profile = launcher.profile;
    let own_pick = if lobby_picks_character_id(profile) {
        row.you.then(|| character_pick(launcher, viewer_is_guest))
    } else {
        row.picker.then(|| body_pick(profile, row.body))
    };
    let character = match &own_pick {
        Some(pick) if pick.steppable() => CharacterCell::Stepper(pick.label),
        Some(pick) => CharacterCell::Fixed(pick.label.to_owned()),
        None if lobby_picks_character_id(profile) => {
            let label = launcher
                .partner_character_id(viewer_is_guest)
                .map_or(PARTNER_CHARACTER_UNSEEN_TEXT, |id| {
                    character_label_of(profile, id)
                });
            CharacterCell::Fixed(label.to_owned())
        }
        None => CharacterCell::Fixed(body_label(row.body).to_owned()),
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
        own_pick,
    )
}

fn open_ledger() -> SeatLedger {
    SeatLedger {
        character: CharacterCell::Empty,
        status: None,
        ping: PingCell::Unknown,
    }
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

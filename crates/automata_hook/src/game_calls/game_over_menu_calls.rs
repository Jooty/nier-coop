use automata_game::address::{
    CAMERA_MODE_REQUEST_FLAG_WORD_VALUE, GAME_OVER_MENU_OBJECT, RESTORE_POINT_POINTER_VALUE,
};
use automata_game::defeat::RestorePoint;
use automata_game::load_gate::StoryPlace;
use automata_game::partner::{BODY_HEALTH_OFFSET, BODY_STATE_OFFSET};
use automata_game::shared_defeat::{
    BODY_FLIGHT_DEFEAT_TIMER_OFFSET, FLIGHT_DEFEAT_TIMER_THAT_NEVER_ASKS,
    GAME_OVER_MENU_CHOOSING_SUB_STATE, GAME_OVER_MENU_CURSOR_OFFSET, GAME_OVER_MENU_FADE_OUT_STATE,
    GAME_OVER_MENU_RETRY_CHOSEN_STATE, GAME_OVER_MENU_RETRY_CURSOR, GAME_OVER_MENU_STATE_OFFSET,
    GAME_OVER_MENU_SUB_STATE_OFFSET, GAME_OVER_MENU_TITLE_CURSOR, GameOverMenu, IN_PLACE_RETRY_BIT,
    PLACE_RECORD_HAS_A_POINT_FLAG, PLACE_RECORD_PHASE_ID_OFFSET,
    PLACE_RECORD_SUB_PHASE_NAME_OFFSET, PartnerBodyReading, RESTORE_POINT_PLACE_RECORD_OFFSET,
    RestorePlace,
};
use automata_game::start::{SUB_PHASE_NAME_BUFFER_LEN, SubPhaseReading};

use crate::game_calls::GameCalls;

impl GameCalls {
    pub fn read_game_over_menu(&self) -> Option<GameOverMenu> {
        let menu = self.address_value(GAME_OVER_MENU_OBJECT);
        Some(GameOverMenu {
            done: self.read_i32_at(menu).ok()?,
            state: self.read_i32_at(menu + GAME_OVER_MENU_STATE_OFFSET).ok()?,
            sub_state: self
                .read_i32_at(menu + GAME_OVER_MENU_SUB_STATE_OFFSET)
                .ok()?,
            cursor: self.read_i32_at(menu + GAME_OVER_MENU_CURSOR_OFFSET).ok()?,
        })
    }

    /// The bit only the phase class of phase 0xFA0 sets: with it the menu's retry reloads in place.
    pub fn in_place_retry_is_set(&self) -> bool {
        self.read_u32(CAMERA_MODE_REQUEST_FLAG_WORD_VALUE)
            .is_ok_and(|word| word & IN_PLACE_RETRY_BIT != 0)
    }

    /// The place the restore point's placement record names; `None` without a restore point.
    pub fn read_restore_place(&self) -> Option<RestorePlace> {
        if self.read_restore_point() != RestorePoint::Present {
            return None;
        }
        let buffer = self.read_u64(RESTORE_POINT_POINTER_VALUE).ok()?;
        let record = buffer + RESTORE_POINT_PLACE_RECORD_OFFSET;
        let flags = self.read_u32_at(record).ok()?;
        let phase_id = self
            .read_i32_at(record + PLACE_RECORD_PHASE_ID_OFFSET)
            .ok()?;
        let bytes = self
            .read_bytes_at(
                record + PLACE_RECORD_SUB_PHASE_NAME_OFFSET,
                SUB_PHASE_NAME_BUFFER_LEN,
            )
            .ok()?;
        let name = match SubPhaseReading::of_bytes(&bytes) {
            SubPhaseReading::Name(name) => name,
            SubPhaseReading::NotAName => String::new(),
        };
        Some(RestorePlace {
            place: StoryPlace::new(phase_id, &name),
            has_a_point: flags & PLACE_RECORD_HAS_A_POINT_FLAG != 0,
        })
    }

    /// The writes of the menu's own "yes" to its title question: cursor 1, sub-state 2, state 5.
    pub fn write_game_over_menu_title(&self) -> bool {
        let menu = self.address_value(GAME_OVER_MENU_OBJECT);
        [
            (GAME_OVER_MENU_CURSOR_OFFSET, GAME_OVER_MENU_TITLE_CURSOR),
            (
                GAME_OVER_MENU_SUB_STATE_OFFSET,
                GAME_OVER_MENU_CHOOSING_SUB_STATE,
            ),
            (GAME_OVER_MENU_STATE_OFFSET, GAME_OVER_MENU_FADE_OUT_STATE),
        ]
        .into_iter()
        .all(|(offset, value)| self.write_i32_at(menu + offset, value).is_ok())
    }

    /// After the retry's phase request: cursor 0, so the flow ends rather than asking for the
    /// title, and state 6, the menu's own state after its retry.
    pub fn write_game_over_menu_retry_chosen(&self) -> bool {
        let menu = self.address_value(GAME_OVER_MENU_OBJECT);
        [
            (GAME_OVER_MENU_CURSOR_OFFSET, GAME_OVER_MENU_RETRY_CURSOR),
            (
                GAME_OVER_MENU_STATE_OFFSET,
                GAME_OVER_MENU_RETRY_CHOSEN_STATE,
            ),
        ]
        .into_iter()
        .all(|(offset, value)| self.write_i32_at(menu + offset, value).is_ok())
    }

    pub fn read_partner_body_for_the_guard(&self, body: usize) -> Option<PartnerBodyReading> {
        let at = |offset: u32| body as u64 + u64::from(offset);
        Some(PartnerBodyReading {
            state: self.read_i32_at(at(BODY_STATE_OFFSET)).ok()?,
            flight_defeat_timer: self.read_f32_at(at(BODY_FLIGHT_DEFEAT_TIMER_OFFSET)).ok()?,
            health: self.read_i32_at(at(BODY_HEALTH_OFFSET)).ok()?,
        })
    }

    pub fn disarm_the_flight_defeat_timer(&self, body: usize) -> bool {
        self.write_f32_at(
            body as u64 + u64::from(BODY_FLIGHT_DEFEAT_TIMER_OFFSET),
            FLIGHT_DEFEAT_TIMER_THAT_NEVER_ASKS,
        )
        .is_ok()
    }
}

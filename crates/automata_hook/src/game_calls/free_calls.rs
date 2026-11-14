use automata_game::address::{
    CONTROLLER_STOPPING_MODE_VALUE, CURRENT_PHASE_ID_VALUE, EVENT_INPUT_OFF_FLAG_WORD_VALUE,
    GAME_PAUSE_VALUE, SCENE_INPUT_OFF_FLAG_WORD_VALUE, SCENE_LOADED_FLAG_VALUE, SCENE_STATE_VALUE,
};
use automata_game::partner::{
    BODY_FLAGS_OFFSET, BODY_INPUT_BLANK_SWITCH_OFFSET, BODY_INPUT_BLANK_TIMER_OFFSET,
    BODY_PLAYER_CONTROLLED_OFFSET, COMPUTER_CONTROLLED_FLAG_BITS, FreeReadings, GlobalGates,
    HeroGates,
};
use automata_game::start::{PlayReadings, is_in_play};
use automata_game::travel::game_is_paused;

use crate::game_calls::GameCalls;

const SCENE_VALUE_NOT_READ: i32 = -99;

impl GameCalls {
    pub fn read_play(&self, hero_present: bool) -> PlayReadings {
        let value = |address| self.read_i32(address).unwrap_or(SCENE_VALUE_NOT_READ);
        PlayReadings {
            scene_state: value(SCENE_STATE_VALUE),
            phase_id: value(CURRENT_PHASE_ID_VALUE),
            scene_loaded: value(SCENE_LOADED_FLAG_VALUE),
            player_present: hero_present,
        }
    }

    pub fn read_game_is_paused(&self) -> bool {
        game_is_paused(self.read_i32(GAME_PAUSE_VALUE).ok())
    }

    /// Game thread only: asks the game for its hero and reads the gates that blank its input.
    pub fn read_free(&self) -> FreeReadings {
        let hero = self.current_hero();
        FreeReadings {
            in_play: is_in_play(&self.read_play(hero != 0)),
            hero: (hero != 0).then(|| self.read_hero_gates(hero)).flatten(),
            globals: self.read_global_gates(),
        }
    }

    fn read_hero_gates(&self, hero: usize) -> Option<HeroGates> {
        let flags = self
            .read_u32_at(self.body_field(hero, BODY_FLAGS_OFFSET))
            .ok()?;
        let int = |offset| self.read_i32_at(self.body_field(hero, offset)).ok();
        Some(HeroGates {
            player_controlled: int(BODY_PLAYER_CONTROLLED_OFFSET)? != 0,
            computer_controlled: flags & COMPUTER_CONTROLLED_FLAG_BITS != 0,
            input_blank_switch: int(BODY_INPUT_BLANK_SWITCH_OFFSET)?,
            input_blank_seconds: self
                .read_f32_at(self.body_field(hero, BODY_INPUT_BLANK_TIMER_OFFSET))
                .ok()?,
        })
    }

    fn read_global_gates(&self) -> Option<GlobalGates> {
        Some(GlobalGates {
            scene_input_off_word: self.read_u32(SCENE_INPUT_OFF_FLAG_WORD_VALUE).ok()?,
            event_input_off_word: self.read_u32(EVENT_INPUT_OFF_FLAG_WORD_VALUE).ok()?,
            controller_stopping_mode: self.read_i32(CONTROLLER_STOPPING_MODE_VALUE).ok()?,
        })
    }
}

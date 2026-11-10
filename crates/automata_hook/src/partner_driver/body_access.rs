use automata_game::partner::{
    BODY_CONTROLLER_OFFSET, BODY_DISMISSED_FLAG_OFFSET, BODY_ENTITY_POINTER_OFFSET,
    BODY_FACING_OFFSET, BODY_FLAGS_OFFSET, BODY_HEALTH_OFFSET, BODY_HELD_BUTTONS_OFFSET,
    BODY_INPUT_IGNORE_TIMER_OFFSET, BODY_PLAYER_CONTROLLED_OFFSET, BODY_POSITION_OFFSET,
    BODY_PRESSED_BUTTONS_OFFSET, BODY_RUN_SPEED_TYPE_OFFSET, BODY_SPEED_OFFSET, BODY_STATE_OFFSET,
    BODY_WORLD_FACING_OFFSET, BodyFields, BodySnapshot, CONTROLLER_COMPUTER_ENABLE_OFFSET,
    CONTROLLER_PAD_ENABLE_OFFSET, CONTROLLER_SCRIPT_ENABLE_OFFSET, ENTITY_HANDLE_OFFSET,
    RecordedStart, speed_for_wire, state_id_for_wire,
};
use nc_hook::memory::MemoryError;

use crate::game_calls::GameCalls;

impl GameCalls {
    pub(crate) fn body_field(&self, body: usize, offset: u32) -> u64 {
        body as u64 + u64::from(offset)
    }

    pub(super) fn entity_of_body(&self, body: usize) -> Result<usize, MemoryError> {
        Ok(self.read_u64_at(self.body_field(body, BODY_ENTITY_POINTER_OFFSET))? as usize)
    }

    pub(super) fn handle_of_body(&self, body: usize) -> Result<u32, MemoryError> {
        let entity = self.entity_of_body(body)?;
        self.read_u32_at(entity as u64 + u64::from(ENTITY_HANDLE_OFFSET))
    }

    pub(super) fn read_body_fields(&self, body: usize) -> Result<BodyFields, MemoryError> {
        let controller = body + BODY_CONTROLLER_OFFSET as usize;
        let int = |offset| self.read_i32_at(self.body_field(body, offset));
        let enable = |offset| self.read_i32_at(self.body_field(controller, offset));
        Ok(BodyFields {
            flags: self.read_u32_at(self.body_field(body, BODY_FLAGS_OFFSET))?,
            player_controlled: int(BODY_PLAYER_CONTROLLED_OFFSET)?,
            pad_enable: enable(CONTROLLER_PAD_ENABLE_OFFSET)?,
            script_enable: enable(CONTROLLER_SCRIPT_ENABLE_OFFSET)?,
            computer_enable: enable(CONTROLLER_COMPUTER_ENABLE_OFFSET)?,
            input_ignore_seconds: self
                .read_f32_at(self.body_field(body, BODY_INPUT_IGNORE_TIMER_OFFSET))?,
            run_speed_type: int(BODY_RUN_SPEED_TYPE_OFFSET)?,
            dismissed: int(BODY_DISMISSED_FLAG_OFFSET)?,
        })
    }

    pub(super) fn write_body_fields(
        &self,
        body: usize,
        current: &BodyFields,
        wanted: &BodyFields,
    ) -> Result<(), MemoryError> {
        let controller = body + BODY_CONTROLLER_OFFSET as usize;
        // Only fields that differ are written, so a settled body costs no writes.
        if current.flags != wanted.flags {
            self.write_u32_at(self.body_field(body, BODY_FLAGS_OFFSET), wanted.flags)?;
        }
        let ints = [
            (
                body,
                BODY_PLAYER_CONTROLLED_OFFSET,
                current.player_controlled,
                wanted.player_controlled,
            ),
            (
                controller,
                CONTROLLER_PAD_ENABLE_OFFSET,
                current.pad_enable,
                wanted.pad_enable,
            ),
            (
                controller,
                CONTROLLER_SCRIPT_ENABLE_OFFSET,
                current.script_enable,
                wanted.script_enable,
            ),
            (
                controller,
                CONTROLLER_COMPUTER_ENABLE_OFFSET,
                current.computer_enable,
                wanted.computer_enable,
            ),
            (
                body,
                BODY_RUN_SPEED_TYPE_OFFSET,
                current.run_speed_type,
                wanted.run_speed_type,
            ),
            (
                body,
                BODY_DISMISSED_FLAG_OFFSET,
                current.dismissed,
                wanted.dismissed,
            ),
        ];
        for (base, offset, now, then) in ints {
            if now != then {
                self.write_i32_at(self.body_field(base, offset), then)?;
            }
        }
        if current.input_ignore_seconds.to_bits() != wanted.input_ignore_seconds.to_bits() {
            self.write_f32_at(
                self.body_field(body, BODY_INPUT_IGNORE_TIMER_OFFSET),
                wanted.input_ignore_seconds,
            )?;
        }
        Ok(())
    }

    pub(super) fn read_pressed_buttons(&self, body: usize) -> Result<u32, MemoryError> {
        self.read_u32_at(self.body_field(body, BODY_PRESSED_BUTTONS_OFFSET))
    }

    pub(crate) fn read_body_position(&self, body: usize) -> Result<[f32; 4], MemoryError> {
        self.read_f32s_at(self.body_field(body, BODY_POSITION_OFFSET))
    }

    pub(super) fn read_body_snapshot(
        &self,
        body: usize,
        pressed: u32,
        last_start: Option<RecordedStart>,
    ) -> Result<BodySnapshot, MemoryError> {
        let position = self.read_body_position(body)?;
        Ok(BodySnapshot {
            held: self.read_u32_at(self.body_field(body, BODY_HELD_BUTTONS_OFFSET))?,
            pressed,
            speed: speed_for_wire(self.read_f32_at(self.body_field(body, BODY_SPEED_OFFSET))?),
            world_facing: self.read_f32_at(self.body_field(body, BODY_WORLD_FACING_OFFSET))?,
            position: [position[0], position[1], position[2]],
            body_facing: self.read_f32_at(self.body_field(body, BODY_FACING_OFFSET))?,
            state: state_id_for_wire(self.read_i32_at(self.body_field(body, BODY_STATE_OFFSET))?),
            last_start,
            health: self.read_i32_at(self.body_field(body, BODY_HEALTH_OFFSET))?,
            free: false,
            phase_id: 0,
        })
    }
}

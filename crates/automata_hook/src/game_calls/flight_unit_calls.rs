use automata_game::address::{
    END_PHASE_EVENT_FLIGHT_UNIT_FUNCTION, ENTITY_AS_FLIGHT_UNIT_FUNCTION,
    SET_FLIGHT_MODE_FIGHTER_FUNCTION, SET_FLIGHT_MODE_ROBOT_FUNCTION,
    START_PHASE_EVENT_FLIGHT_UNIT_FUNCTION,
};
use automata_game::partner::{
    BODY_CONTROLLER_OFFSET, BODY_HEALTH_OFFSET, BODY_STATE_OFFSET,
    CONTROLLER_KEYBOARD_ENABLE_OFFSET, CONTROLLER_MOUSE_ENABLE_OFFSET,
    CONTROLLER_PAD_ENABLE_OFFSET,
};
use automata_game::partner_flight::{
    BODY_FLIGHT_MODE_FIGHTER_OFFSET, BODY_FLIGHT_UNIT_HANDLE_OFFSET,
    BODY_RIDING_FLIGHT_UNIT_OFFSET, BODY_SHOOTING_STAGE_OFFSET, BodyFlight,
    CONTROLLER_KEYBOARD_STICK_LENGTH_OFFSET, CONTROLLER_KEYBOARD_STICK_OFFSET,
    CONTROLLER_MOUSE_STICK_LENGTH_OFFSET, CONTROLLER_MOUSE_STICK_OFFSET,
    CONTROLLER_PAD_STICK_OFFSET, FLIGHT_UNIT_POSITION_OFFSET, FLIGHT_UNIT_YAW_OFFSET, FlightFields,
    FlightMode, PAD_ENABLED, StickChoice, UnitPose, flight_stick_in_use,
};

use crate::game_calls::GameCalls;

const NO_UNIT: usize = 0;
const NOW: i32 = 1;

type OnePointerCall = unsafe extern "system" fn(usize);
type EntityAsFlightUnit = unsafe extern "system" fn(usize) -> usize;
type SetFlightMode = unsafe extern "system" fn(usize, i32);

impl GameCalls {
    /// The flight fields of a live player body, with its unit and the stick its flight reads.
    pub fn read_flight_fields(&self, body: usize) -> Option<FlightFields> {
        let int = |offset| self.read_i32_at(self.body_field(body, offset)).ok();
        Some(FlightFields {
            riding: int(BODY_RIDING_FLIGHT_UNIT_OFFSET)?,
            shooting_stage: int(BODY_SHOOTING_STAGE_OFFSET)?,
            mode_word: int(BODY_FLIGHT_MODE_FIGHTER_OFFSET)?,
            unit: self.read_unit_pose(self.flight_unit_of(body)),
            stick: self.read_flight_stick(body).unwrap_or_default(),
        })
    }

    /// The partner body's flight fields as the flight rule reads them.
    pub fn read_body_flight(&self, body: usize) -> Option<BodyFlight> {
        let int = |offset| self.read_i32_at(self.body_field(body, offset)).ok();
        Some(BodyFlight {
            riding: int(BODY_RIDING_FLIGHT_UNIT_OFFSET)? != 0,
            mode: FlightMode::of_mode_word(int(BODY_FLIGHT_MODE_FIGHTER_OFFSET)?),
            state: int(BODY_STATE_OFFSET)?,
            alive: int(BODY_HEALTH_OFFSET)? > 0,
        })
    }

    /// The flight unit the body's unit handle names; 0 when the handle is empty or dead.
    #[expect(unsafe_code, reason = "calls the game's flight unit cast")]
    pub fn flight_unit_of(&self, body: usize) -> usize {
        let Ok(handle) = self.read_u32_at(self.body_field(body, BODY_FLIGHT_UNIT_HANDLE_OFFSET))
        else {
            return NO_UNIT;
        };
        if handle == 0 {
            return NO_UNIT;
        }
        let entity = self.entity_of_handle(handle);
        if entity == 0 {
            return NO_UNIT;
        }
        // SAFETY: the address is Entity::asFlightUnit of the verified build; the entity was just
        // resolved from a live handle, and the cast returns 0 for another class.
        unsafe {
            core::mem::transmute::<usize, EntityAsFlightUnit>(
                self.address_of(ENTITY_AS_FLIGHT_UNIT_FUNCTION),
            )(entity)
        }
    }

    pub fn read_unit_pose(&self, unit: usize) -> Option<UnitPose> {
        if unit == NO_UNIT {
            return None;
        }
        let position = self
            .read_f32s_at::<3>(unit as u64 + u64::from(FLIGHT_UNIT_POSITION_OFFSET))
            .ok()?;
        let yaw = self
            .read_f32_at(unit as u64 + u64::from(FLIGHT_UNIT_YAW_OFFSET))
            .ok()?;
        Some(UnitPose { position, yaw })
    }

    pub fn write_unit_position(&self, unit: usize, position: [f32; 3]) -> bool {
        self.write_f32s_at(
            unit as u64 + u64::from(FLIGHT_UNIT_POSITION_OFFSET),
            &position,
        )
        .is_ok()
    }

    pub fn write_unit_yaw(&self, unit: usize, yaw: f32) -> bool {
        self.write_f32_at(unit as u64 + u64::from(FLIGHT_UNIT_YAW_OFFSET), yaw)
            .is_ok()
    }

    /// Opens the pad gate of the body's controller block, the one its flight movement reads the
    /// pad stick by.
    pub fn open_flight_pad_gate(&self, body: usize) -> bool {
        let controller = body + BODY_CONTROLLER_OFFSET as usize;
        self.write_i32_at(
            self.body_field(controller, CONTROLLER_PAD_ENABLE_OFFSET),
            PAD_ENABLED,
        )
        .is_ok()
    }

    /// # Safety
    /// `body` must be a live, alive player body resolved this tick on the game thread.
    #[expect(unsafe_code, reason = "calls the game's flight unit start")]
    pub unsafe fn start_phase_event_flight_unit(&self, body: usize) {
        // SAFETY: the address is Pl0000::startPhaseEventFlightUnit of the verified build, which
        // takes only the body; the caller upholds the contract above.
        unsafe {
            core::mem::transmute::<usize, OnePointerCall>(
                self.address_of(START_PHASE_EVENT_FLIGHT_UNIT_FUNCTION),
            )(body);
        }
    }

    /// # Safety
    /// `body` must be a live player body resolved this tick on the game thread.
    #[expect(unsafe_code, reason = "calls the game's flight unit end")]
    pub unsafe fn end_phase_event_flight_unit(&self, body: usize) {
        // SAFETY: the address is Pl0000::endPhaseEventFlightUnit of the verified build, which
        // takes only the body; the caller upholds the contract above.
        unsafe {
            core::mem::transmute::<usize, OnePointerCall>(
                self.address_of(END_PHASE_EVENT_FLIGHT_UNIT_FUNCTION),
            )(body);
        }
    }

    /// # Safety
    /// `body` must be a live, alive player body that rides a flight unit, resolved this tick on
    /// the game thread. A mode of none calls nothing.
    #[expect(unsafe_code, reason = "calls the game's flight mode setter")]
    pub unsafe fn set_flight_mode(&self, body: usize, mode: FlightMode) {
        let setter = match mode {
            FlightMode::Fighter => SET_FLIGHT_MODE_FIGHTER_FUNCTION,
            FlightMode::Robot => SET_FLIGHT_MODE_ROBOT_FUNCTION,
            FlightMode::None => return,
        };
        // SAFETY: the address is one of the two Pl0000 flight mode setters of the verified build;
        // with its second argument set it writes the mode and starts state 0x84 on the body. The
        // caller upholds the contract above.
        unsafe {
            core::mem::transmute::<usize, SetFlightMode>(self.address_of(setter))(body, NOW);
        }
    }

    fn read_flight_stick(&self, body: usize) -> Option<[f32; 2]> {
        let controller = body + BODY_CONTROLLER_OFFSET as usize;
        let field = |offset| self.body_field(controller, offset);
        let int = |offset| self.read_i32_at(field(offset)).ok();
        let float = |offset| self.read_f32_at(field(offset)).ok();
        let vector = |offset| self.read_f32s_at::<2>(field(offset)).ok();
        Some(flight_stick_in_use(&StickChoice {
            pad_enable: int(CONTROLLER_PAD_ENABLE_OFFSET)?,
            mouse_enable: int(CONTROLLER_MOUSE_ENABLE_OFFSET)?,
            keyboard_enable: int(CONTROLLER_KEYBOARD_ENABLE_OFFSET)?,
            mouse_length: float(CONTROLLER_MOUSE_STICK_LENGTH_OFFSET)?,
            keyboard_length: float(CONTROLLER_KEYBOARD_STICK_LENGTH_OFFSET)?,
            pad: vector(CONTROLLER_PAD_STICK_OFFSET)?,
            mouse: vector(CONTROLLER_MOUSE_STICK_OFFSET)?,
            keyboard: vector(CONTROLLER_KEYBOARD_STICK_OFFSET)?,
        }))
    }
}

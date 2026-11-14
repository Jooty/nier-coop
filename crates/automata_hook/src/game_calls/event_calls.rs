use automata_game::address::{
    EVENT_CURRENT_FRAME_FUNCTION, EVENT_IS_PAUSED_FUNCTION, EVENT_LAST_FRAME_FUNCTION,
    EVENT_LOAD_AND_PLAY_FUNCTION, EVENT_LOAD_FUNCTION, EVENT_RELEASE_FUNCTION,
    EVENT_SET_PAUSED_FUNCTION, EVENT_SKIP_FUNCTION, EVENT_SLOT_TABLE, GAME_PAUSE_VALUE,
    GameAddress, PHASE_EVENT_ACTION_VTABLE, UI_BOOKKEEPING_WORD_VALUE,
};
use automata_game::event_follow::{
    EVENT_FLAG_WORD_PLAYING_MASK, EVENT_ID_BYTES, EVENT_SLOT_PLAY_WANTED_BIT, EVENT_SLOT_STRIDE,
    EVENT_SLOT_TABLE_BYTES, EventId, EventProgress, EventSlot, PHASE_EVENT_ACTION_DATA_ID_OFFSET,
    slots_of_table,
};

use crate::game_calls::GameCalls;

type DataIdCall = unsafe extern "system" fn(usize) -> usize;
type IntArgumentCall = unsafe extern "system" fn(i32);
type NoArgumentCall = unsafe extern "system" fn() -> usize;
type FrameOfCall = unsafe extern "system" fn(usize) -> f32;
type CountOfCall = unsafe extern "system" fn(usize) -> i32;
type FlagCall = unsafe extern "system" fn() -> i32;

impl GameCalls {
    pub fn event_slots(&self) -> Vec<EventSlot> {
        self.read_bytes(EVENT_SLOT_TABLE, EVENT_SLOT_TABLE_BYTES)
            .map(|table| slots_of_table(&table))
            .unwrap_or_default()
    }

    pub fn event_flags_say_playing(&self) -> bool {
        self.read_u32(UI_BOOKKEEPING_WORD_VALUE)
            .is_ok_and(|word| word & EVENT_FLAG_WORD_PLAYING_MASK != 0)
    }

    pub fn pause_state(&self) -> Option<i32> {
        self.read_i32(GAME_PAUSE_VALUE).ok()
    }

    pub fn read_event_id_at(&self, address: u64) -> Option<EventId> {
        let bytes = self.read_bytes_at(address, EVENT_ID_BYTES).ok()?;
        EventId::of_bytes(&bytes)
    }

    /// True when the DataId lies inside a story graph event action (the script's `.play`).
    pub fn is_a_phase_event_actions_data_id(&self, data_id_address: u64) -> bool {
        let Some(action) = data_id_address.checked_sub(PHASE_EVENT_ACTION_DATA_ID_OFFSET) else {
            return false;
        };
        self.read_u64_at(action)
            .is_ok_and(|vtable| vtable == self.address_value(PHASE_EVENT_ACTION_VTABLE))
    }

    /// Clears the play-wanted bit of the live slot holding this event; false when none does.
    pub fn hold_slot_play(&self, id: EventId) -> bool {
        let Ok(table) = self.read_bytes(EVENT_SLOT_TABLE, EVENT_SLOT_TABLE_BYTES) else {
            return false;
        };
        let base = self.address_value(EVENT_SLOT_TABLE);
        let Some(index) = table
            .as_chunks::<EVENT_SLOT_STRIDE>()
            .0
            .iter()
            .position(|bytes| {
                EventSlot::of_bytes(bytes).is_some_and(|slot| slot.is_live() && slot.id == id)
            })
        else {
            return false;
        };
        let at = base + (index * EVENT_SLOT_STRIDE) as u64;
        self.read_u32_at(at)
            .and_then(|flags| self.write_u32_at(at, flags & !EVENT_SLOT_PLAY_WANTED_BIT))
            .is_ok()
    }

    #[expect(
        unsafe_code,
        reason = "calls the event player's frame and pause readers"
    )]
    pub fn event_progress(&self, slots: &[EventSlot]) -> Option<EventProgress> {
        let playing = slots.iter().find(|slot| slot.is_playing())?;
        let data_id = playing.id.to_bytes();
        let argument = data_id.as_ptr() as usize;
        // SAFETY: the three addresses are event player functions of the verified build; the
        // first two take one `const DataId*` of twelve bytes that outlive the calls, the third
        // takes nothing and reads the player's pause flag. The caller is the game thread.
        let (frame, last_frame, paused) = unsafe {
            (
                core::mem::transmute::<usize, FrameOfCall>(
                    self.address_of(EVENT_CURRENT_FRAME_FUNCTION),
                )(argument),
                core::mem::transmute::<usize, CountOfCall>(
                    self.address_of(EVENT_LAST_FRAME_FUNCTION),
                )(argument),
                core::mem::transmute::<usize, FlagCall>(self.address_of(EVENT_IS_PAUSED_FUNCTION))(
                ),
            )
        };
        Some(EventProgress {
            id: playing.id,
            frame: frame as i32,
            last_frame,
            paused: paused != 0,
        })
    }

    pub fn load_event(&self, id: EventId) {
        self.call_with_data_id(EVENT_LOAD_FUNCTION, id);
    }

    pub fn play_event(&self, id: EventId) {
        self.call_with_data_id(EVENT_LOAD_AND_PLAY_FUNCTION, id);
    }

    pub fn release_event(&self, id: EventId) {
        self.call_with_data_id(EVENT_RELEASE_FUNCTION, id);
    }

    /// Calls the game's skip only while a slot plays: with none the skip would still set its flags.
    pub fn skip_playing_events(&self) -> bool {
        if !self.event_slots().iter().any(EventSlot::is_playing) {
            return false;
        }
        self.skip_events();
        true
    }

    #[expect(
        unsafe_code,
        reason = "calls an event player function of the game with a DataId"
    )]
    fn call_with_data_id(&self, function: GameAddress, id: EventId) {
        let data_id = id.to_bytes();
        // SAFETY: the address is an event player function of the verified build taking one
        // `const DataId*`; it reads (or copies) the twelve bytes, which outlive the call. The
        // caller is the game thread.
        unsafe {
            core::mem::transmute::<usize, DataIdCall>(self.address_of(function))(
                data_id.as_ptr() as usize
            );
        }
    }

    pub fn set_event_clock_paused(&self, paused: bool) {
        self.call_set_paused(i32::from(paused));
    }

    #[expect(unsafe_code, reason = "calls the event player's pause function")]
    fn call_set_paused(&self, paused: i32) {
        // SAFETY: the address is the event player's SetPaused of the verified build, which takes
        // one int and pauses or resumes every playing slot; the caller is the game thread.
        unsafe {
            core::mem::transmute::<usize, IntArgumentCall>(
                self.address_of(EVENT_SET_PAUSED_FUNCTION),
            )(paused);
        }
    }

    #[expect(unsafe_code, reason = "calls the game's event skip function")]
    fn skip_events(&self) {
        // SAFETY: the address is the event skip function of the verified build, which takes no
        // argument (the script's `.stop` calls it the same way); the caller is the game thread.
        unsafe {
            core::mem::transmute::<usize, NoArgumentCall>(self.address_of(EVENT_SKIP_FUNCTION))();
        }
    }
}

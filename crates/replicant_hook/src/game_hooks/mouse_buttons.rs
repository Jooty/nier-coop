use core::ffi::c_void;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

const BUTTONS_OFFSET_IN_STATE: usize = 12;
const MIN_STATE_BYTES_WITH_TWO_BUTTONS: u32 = 14;
const OFFSET_OF_BUTTON_LEFT: u32 = 12;
const OFFSET_OF_BUTTON_RIGHT: u32 = 13;
const BUTTON_DOWN_BIT: u8 = 0x80;
const LEFT_BIT: u8 = 1;
const RIGHT_BIT: u8 = 2;

static DIRECT_INPUT_BUTTON_BITS: AtomicU8 = AtomicU8::new(0);
static DIRECT_INPUT_DOWN_EVENTS: AtomicU32 = AtomicU32::new(0);

pub fn direct_input_buttons() -> [bool; 2] {
    let bits = DIRECT_INPUT_BUTTON_BITS.load(Ordering::Relaxed);
    [bits & LEFT_BIT != 0, bits & RIGHT_BIT != 0]
}

pub fn direct_input_down_events() -> u32 {
    DIRECT_INPUT_DOWN_EVENTS.load(Ordering::Relaxed)
}

pub(super) fn note_state(data: *const c_void, size: u32) {
    if data.is_null() || size < MIN_STATE_BYTES_WITH_TWO_BUTTONS {
        return;
    }
    let mut bits = 0;
    for (index, bit) in [LEFT_BIT, RIGHT_BIT].into_iter().enumerate() {
        #[expect(
            unsafe_code,
            reason = "reads the button bytes of the game's mouse state"
        )]
        // SAFETY: the game passed `size` readable bytes and `size` covers both button bytes.
        let button = unsafe { *data.cast::<u8>().add(BUTTONS_OFFSET_IN_STATE + index) };
        if button & BUTTON_DOWN_BIT != 0 {
            bits |= bit;
        }
    }
    let previous = DIRECT_INPUT_BUTTON_BITS.swap(bits, Ordering::Relaxed);
    if bits & !previous != 0 {
        DIRECT_INPUT_DOWN_EVENTS.fetch_add(1, Ordering::Relaxed);
    }
}

pub(super) fn note_events(elements: *const c_void, element_size: u32, count: u32) {
    let minimum_element_size = 8;
    if elements.is_null() || element_size < minimum_element_size {
        return;
    }
    for index in 0..count as usize {
        #[expect(unsafe_code, reason = "reads one buffered mouse event")]
        // SAFETY: the game's buffer holds `count` elements of `element_size` bytes each, and the
        // first two fields of each are the offset and the data.
        let (offset, data) = unsafe {
            let element = elements.cast::<u8>().add(index * element_size as usize);
            (
                element.cast::<u32>().read_unaligned(),
                element.add(4).cast::<u32>().read_unaligned(),
            )
        };
        let bit = match offset {
            OFFSET_OF_BUTTON_LEFT => LEFT_BIT,
            OFFSET_OF_BUTTON_RIGHT => RIGHT_BIT,
            _ => continue,
        };
        if data & u32::from(BUTTON_DOWN_BIT) != 0 {
            DIRECT_INPUT_BUTTON_BITS.fetch_or(bit, Ordering::Relaxed);
            DIRECT_INPUT_DOWN_EVENTS.fetch_add(1, Ordering::Relaxed);
        } else {
            DIRECT_INPUT_BUTTON_BITS.fetch_and(!bit, Ordering::Relaxed);
        }
    }
}

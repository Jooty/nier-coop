use core::ffi::c_void;
use core::ptr;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::sync::{Mutex, MutexGuard, PoisonError};

use rc_game::harness::KEY_CODES;
use windows_sys::Win32::System::SystemInformation::GetTickCount;

const KEY_STATE_BYTES: u32 = 256;
const KEY_COUNT: usize = 256;
const DOWN_BIT: u8 = 0x80;
const MOUSE_BUTTONS_OFFSET_IN_STATE: usize = 12;
const MOUSE_BUTTON_COUNT: usize = 2;
const MOUSE_FIRST_BUTTON_EVENT_OFFSET: u32 = 12;
const MAX_QUEUED_EVENTS: usize = 256;
const MIN_EVENT_SIZE: u32 = 16;
const EVENT_DATA_OFFSET: usize = 4;
const EVENT_TIMESTAMP_OFFSET: usize = 8;
const EVENT_SEQUENCE_OFFSET: usize = 12;
const PEEK_FLAG: u32 = 1;

static KEY_DOWN: [AtomicBool; KEY_COUNT] = [const { AtomicBool::new(false) }; KEY_COUNT];
static MOUSE_BUTTON_BITS: AtomicU8 = AtomicU8::new(0);
static EVENT_SEQUENCE: AtomicU32 = AtomicU32::new(0);
static KEYBOARD_EVENTS: Mutex<VecDeque<QueuedEvent>> = Mutex::new(VecDeque::new());
static MOUSE_EVENTS: Mutex<VecDeque<QueuedEvent>> = Mutex::new(VecDeque::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectedDevice {
    Keyboard,
    Mouse,
}

#[derive(Debug, Clone, Copy)]
struct QueuedEvent {
    offset: u32,
    data: u32,
}

pub(super) fn set_key(scan_code: u8, down: bool) {
    let was_down = KEY_DOWN[usize::from(scan_code)].swap(down, Ordering::Relaxed);
    if was_down != down {
        queue_event(
            &KEYBOARD_EVENTS,
            QueuedEvent {
                offset: u32::from(scan_code),
                data: if down { u32::from(DOWN_BIT) } else { 0 },
            },
        );
    }
}

pub(super) fn release_all_keys() {
    for scan_code in 0..=u8::MAX {
        set_key(scan_code, false);
    }
}

pub(super) fn set_mouse_button(index: usize, down: bool) {
    if index >= MOUSE_BUTTON_COUNT {
        return;
    }
    let bit = 1_u8 << index;
    let before = if down {
        MOUSE_BUTTON_BITS.fetch_or(bit, Ordering::Relaxed)
    } else {
        MOUSE_BUTTON_BITS.fetch_and(!bit, Ordering::Relaxed)
    };
    if (before & bit != 0) != down {
        queue_event(
            &MOUSE_EVENTS,
            QueuedEvent {
                offset: MOUSE_FIRST_BUTTON_EVENT_OFFSET + index as u32,
                data: if down { u32::from(DOWN_BIT) } else { 0 },
            },
        );
    }
}

pub fn virtual_key_is_down(virtual_key: u8) -> bool {
    KEY_CODES.iter().any(|key| {
        key.virtual_key == virtual_key
            && KEY_DOWN[usize::from(key.direct_input_scan_code)].load(Ordering::Relaxed)
    })
}

#[expect(
    unsafe_code,
    reason = "marks injected inputs in the game's device state buffer"
)]
pub fn patch_state(device: InjectedDevice, data: *mut c_void, size: u32) {
    if data.is_null() {
        return;
    }
    match device {
        InjectedDevice::Keyboard if size >= KEY_STATE_BYTES => {
            for (scan_code, down) in KEY_DOWN.iter().enumerate() {
                if down.load(Ordering::Relaxed) {
                    // SAFETY: the game passed `size` (at least 256) writable bytes.
                    unsafe { *data.cast::<u8>().add(scan_code) |= DOWN_BIT };
                }
            }
        }
        InjectedDevice::Mouse
            if size as usize >= MOUSE_BUTTONS_OFFSET_IN_STATE + MOUSE_BUTTON_COUNT =>
        {
            let bits = MOUSE_BUTTON_BITS.load(Ordering::Relaxed);
            for index in 0..MOUSE_BUTTON_COUNT {
                if bits & (1 << index) != 0 {
                    // SAFETY: the game passed `size` writable bytes and `size` covers both buttons.
                    unsafe {
                        *data.cast::<u8>().add(MOUSE_BUTTONS_OFFSET_IN_STATE + index) |= DOWN_BIT;
                    }
                }
            }
        }
        _ => {}
    }
}

#[expect(
    unsafe_code,
    reason = "appends injected events to the game's buffered device data"
)]
pub fn extend_events(
    device: InjectedDevice,
    elements: *mut c_void,
    element_size: u32,
    element_count: *mut u32,
    capacity: u32,
    flags: u32,
) {
    if elements.is_null() || element_count.is_null() || element_size < MIN_EVENT_SIZE {
        return;
    }
    let queue = match device {
        InjectedDevice::Keyboard => &KEYBOARD_EVENTS,
        InjectedDevice::Mouse => &MOUSE_EVENTS,
    };
    let mut pending = lock(queue);
    // SAFETY: the game passed a readable and writable in-out count.
    let already = unsafe { element_count.read() };
    let room = capacity.saturating_sub(already) as usize;
    let taken = room.min(pending.len());
    for (position, event) in pending.iter().take(taken).enumerate() {
        // SAFETY: `capacity` elements of `element_size` bytes were writable and `already + taken` fits.
        unsafe {
            let slot = elements
                .cast::<u8>()
                .add((already as usize + position) * element_size as usize);
            ptr::write_bytes(slot, 0, element_size as usize);
            slot.cast::<u32>().write_unaligned(event.offset);
            slot.add(EVENT_DATA_OFFSET)
                .cast::<u32>()
                .write_unaligned(event.data);
            slot.add(EVENT_TIMESTAMP_OFFSET)
                .cast::<u32>()
                .write_unaligned(GetTickCount());
            slot.add(EVENT_SEQUENCE_OFFSET)
                .cast::<u32>()
                .write_unaligned(EVENT_SEQUENCE.fetch_add(1, Ordering::Relaxed));
        }
    }
    if flags & PEEK_FLAG == 0 {
        pending.drain(..taken);
    }
    if taken > 0 {
        // SAFETY: the game passed a writable in-out count.
        unsafe { element_count.write(already + taken as u32) };
    }
}

fn queue_event(queue: &Mutex<VecDeque<QueuedEvent>>, event: QueuedEvent) {
    let mut pending = lock(queue);
    if pending.len() == MAX_QUEUED_EVENTS {
        pending.pop_front();
    }
    pending.push_back(event);
}

fn lock(queue: &Mutex<VecDeque<QueuedEvent>>) -> MutexGuard<'_, VecDeque<QueuedEvent>> {
    queue.lock().unwrap_or_else(PoisonError::into_inner)
}

use core::ffi::c_void;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, VK_LBUTTON, VK_RBUTTON,
};
use windows_sys::Win32::UI::Input::{GetRawInputData, RAWINPUT, RAWINPUTHEADER};

use crate::game_hooks;

pub(super) const SOURCE_NAMES: [&str; SOURCE_COUNT] = [
    "wm_message",
    "raw_input",
    "direct_input",
    "async_key_state",
    "key_state",
];
const SOURCE_COUNT: usize = 5;
const WM_SOURCE: usize = 0;
const RAW_SOURCE: usize = 1;
const DIRECT_INPUT_SOURCE: usize = 2;
const ASYNC_SOURCE: usize = 3;
const KEY_STATE_SOURCE: usize = 4;

const WM_LBUTTONDOWN: u32 = 0x0201;
const WM_LBUTTONUP: u32 = 0x0202;
const WM_LBUTTONDBLCLK: u32 = 0x0203;
const WM_RBUTTONDOWN: u32 = 0x0204;
const WM_RBUTTONUP: u32 = 0x0205;
const WM_RBUTTONDBLCLK: u32 = 0x0206;
const WM_INPUT: u32 = 0x00FF;
const RAW_INPUT_COMMAND_INPUT: u32 = 0x1000_0003;
const RAW_INPUT_TYPE_MOUSE: u32 = 0;
const RAW_INPUT_FOREGROUND_CODE: usize = 0;
const RAW_LEFT_DOWN: u16 = 0x1;
const RAW_LEFT_UP: u16 = 0x2;
const RAW_RIGHT_DOWN: u16 = 0x4;
const RAW_RIGHT_UP: u16 = 0x8;
const LEFT_BIT: u8 = 1;
const RIGHT_BIT: u8 = 2;

static MESSAGE_BITS: AtomicU8 = AtomicU8::new(0);
static RAW_BITS: AtomicU8 = AtomicU8::new(0);
static POLLED_ASYNC_BITS: AtomicU8 = AtomicU8::new(0);
static POLLED_KEY_STATE_BITS: AtomicU8 = AtomicU8::new(0);
static DOWN_EVENTS: [AtomicU32; SOURCE_COUNT] = [const { AtomicU32::new(0) }; SOURCE_COUNT];

#[derive(Debug, Default, Clone)]
pub(super) struct ClickSources {
    pub(super) down: [[bool; 2]; SOURCE_COUNT],
    pub(super) down_events: [u32; SOURCE_COUNT],
}

impl ClickSources {
    pub(super) fn combined(&self) -> [bool; 2] {
        [
            self.down.iter().any(|source| source[0]),
            self.down.iter().any(|source| source[1]),
        ]
    }
}

pub(super) fn observe_message(umsg: u32, wparam: usize, lparam: isize) -> bool {
    match umsg {
        WM_LBUTTONDOWN | WM_LBUTTONDBLCLK => set_bit(&MESSAGE_BITS, WM_SOURCE, LEFT_BIT, true),
        WM_LBUTTONUP => set_bit(&MESSAGE_BITS, WM_SOURCE, LEFT_BIT, false),
        WM_RBUTTONDOWN | WM_RBUTTONDBLCLK => set_bit(&MESSAGE_BITS, WM_SOURCE, RIGHT_BIT, true),
        WM_RBUTTONUP => set_bit(&MESSAGE_BITS, WM_SOURCE, RIGHT_BIT, false),
        WM_INPUT => {
            observe_raw_input(wparam, lparam);
            return false;
        }
        _ => return false,
    }
    true
}

pub(super) fn sample() -> ClickSources {
    let async_bits = poll_async_bits();
    let key_state_bits = poll_key_state_bits();
    count_new_downs(ASYNC_SOURCE, &POLLED_ASYNC_BITS, async_bits);
    count_new_downs(KEY_STATE_SOURCE, &POLLED_KEY_STATE_BITS, key_state_bits);
    let mut sources = ClickSources::default();
    sources.down[WM_SOURCE] = from_bits(MESSAGE_BITS.load(Ordering::Relaxed));
    sources.down[RAW_SOURCE] = from_bits(RAW_BITS.load(Ordering::Relaxed));
    sources.down[DIRECT_INPUT_SOURCE] = game_hooks::direct_input_buttons();
    sources.down[ASYNC_SOURCE] = from_bits(async_bits);
    sources.down[KEY_STATE_SOURCE] = from_bits(key_state_bits);
    for (index, count) in DOWN_EVENTS.iter().enumerate() {
        sources.down_events[index] = count.load(Ordering::Relaxed);
    }
    sources.down_events[DIRECT_INPUT_SOURCE] = game_hooks::direct_input_down_events();
    sources
}

fn set_bit(bits: &AtomicU8, source: usize, bit: u8, down: bool) {
    if down {
        if bits.fetch_or(bit, Ordering::Relaxed) & bit == 0 {
            DOWN_EVENTS[source].fetch_add(1, Ordering::Relaxed);
        }
    } else {
        bits.fetch_and(!bit, Ordering::Relaxed);
    }
}

fn from_bits(bits: u8) -> [bool; 2] {
    [bits & LEFT_BIT != 0, bits & RIGHT_BIT != 0]
}

fn bits_from(left: bool, right: bool) -> u8 {
    (u8::from(left) * LEFT_BIT) | (u8::from(right) * RIGHT_BIT)
}

#[expect(unsafe_code, reason = "reads the mouse buttons' physical state")]
fn poll_async_bits() -> u8 {
    // SAFETY: GetAsyncKeyState has no preconditions.
    let (left, right) = unsafe {
        (
            GetAsyncKeyState(i32::from(VK_LBUTTON)),
            GetAsyncKeyState(i32::from(VK_RBUTTON)),
        )
    };
    bits_from(left < 0, right < 0)
}

#[expect(unsafe_code, reason = "reads the mouse buttons' thread-queue state")]
fn poll_key_state_bits() -> u8 {
    // SAFETY: GetKeyState has no preconditions.
    let (left, right) = unsafe {
        (
            GetKeyState(i32::from(VK_LBUTTON)),
            GetKeyState(i32::from(VK_RBUTTON)),
        )
    };
    bits_from(left < 0, right < 0)
}

fn count_new_downs(source: usize, previous: &AtomicU8, now: u8) {
    let before = previous.swap(now, Ordering::Relaxed);
    if now & !before != 0 {
        DOWN_EVENTS[source].fetch_add(1, Ordering::Relaxed);
    }
}

#[expect(
    unsafe_code,
    reason = "reads the raw input record of a WM_INPUT message"
)]
fn observe_raw_input(wparam: usize, lparam: isize) {
    if wparam & 0xFF != RAW_INPUT_FOREGROUND_CODE {
        return;
    }
    // SAFETY: an all-zero RAWINPUT is a valid value for this plain data struct.
    let mut record: RAWINPUT = unsafe { core::mem::zeroed() };
    let mut size = size_of::<RAWINPUT>() as u32;
    // SAFETY: the handle is the lparam of a WM_INPUT message and the buffer is `size` bytes.
    let read = unsafe {
        GetRawInputData(
            std::ptr::with_exposed_provenance_mut::<c_void>(lparam as usize),
            RAW_INPUT_COMMAND_INPUT,
            (&raw mut record).cast::<c_void>(),
            &raw mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if read == u32::MAX || record.header.dwType != RAW_INPUT_TYPE_MOUSE {
        return;
    }
    // SAFETY: the header says this record holds mouse data.
    let flags = unsafe { record.data.mouse.Anonymous.Anonymous.usButtonFlags };
    if flags & RAW_LEFT_DOWN != 0 {
        set_bit(&RAW_BITS, RAW_SOURCE, LEFT_BIT, true);
    }
    if flags & RAW_LEFT_UP != 0 {
        set_bit(&RAW_BITS, RAW_SOURCE, LEFT_BIT, false);
    }
    if flags & RAW_RIGHT_DOWN != 0 {
        set_bit(&RAW_BITS, RAW_SOURCE, RIGHT_BIT, true);
    }
    if flags & RAW_RIGHT_UP != 0 {
        set_bit(&RAW_BITS, RAW_SOURCE, RIGHT_BIT, false);
    }
}

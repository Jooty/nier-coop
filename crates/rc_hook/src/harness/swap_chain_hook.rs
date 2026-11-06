use core::ffi::c_void;
use core::ptr;
use std::panic;
use std::sync::atomic::{AtomicPtr, Ordering};

use hudhook::hooks::DummyHwnd;
use hudhook::windows::Win32::Foundation::HMODULE;
use hudhook::windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0,
};
use hudhook::windows::Win32::Graphics::Direct3D11::{
    D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION, D3D11CreateDeviceAndSwapChain, ID3D11Device,
    ID3D11DeviceContext, ID3D11Texture2D,
};
use hudhook::windows::Win32::Graphics::Dxgi::Common::{
    DXGI_FORMAT_R8G8B8A8_UNORM, DXGI_MODE_DESC, DXGI_MODE_SCALING_UNSPECIFIED,
    DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED, DXGI_SAMPLE_DESC,
};
use hudhook::windows::Win32::Graphics::Dxgi::{
    DXGI_SWAP_CHAIN_DESC, DXGI_SWAP_EFFECT_DISCARD, DXGI_USAGE_RENDER_TARGET_OUTPUT, IDXGISwapChain,
};
use hudhook::windows::core::{BOOL, Interface};

use crate::diagnostics::log_line;
use crate::game_hooks::hook_address;
use crate::harness::screenshot::{capture_if_requested, shot_is_pending};

const BACKBUFFER_INDEX: u32 = 0;
const FAILED_HRESULT: i32 = -2_147_467_259;

static ORIGINAL_GET_BUFFER: AtomicPtr<c_void> = AtomicPtr::new(ptr::null_mut());

type GetBuffer =
    unsafe extern "system" fn(*mut c_void, u32, *const c_void, *mut *mut c_void) -> i32;

pub fn install_screenshot_hook() {
    let Some(address) = find_get_buffer_address() else {
        log_line("harness: cannot find IDXGISwapChain::GetBuffer, screenshots are off");
        return;
    };
    #[expect(
        unsafe_code,
        reason = "hooks IDXGISwapChain::GetBuffer to see the finished frame"
    )]
    // SAFETY: the address is slot 9 of a live swap chain's table, whose signature is
    // (this, index, iid, out pointer) returning an HRESULT, matching the replacement.
    unsafe {
        hook_address(
            "IDXGISwapChain::GetBuffer",
            address,
            get_buffer_detour as *const () as *mut c_void,
            &ORIGINAL_GET_BUFFER,
        )
    };
}

#[expect(
    unsafe_code,
    reason = "calls the original GetBuffer through the trampoline MinHook built"
)]
extern "system" fn get_buffer_detour(
    swap_chain: *mut c_void,
    buffer: u32,
    iid: *const c_void,
    surface: *mut *mut c_void,
) -> i32 {
    let original = ORIGINAL_GET_BUFFER.load(Ordering::Acquire);
    if original.is_null() {
        return FAILED_HRESULT;
    }
    // SAFETY: the trampoline continues into the original, which has this exact signature.
    let result = unsafe {
        core::mem::transmute::<*mut c_void, GetBuffer>(original)(swap_chain, buffer, iid, surface)
    };
    if result >= 0 && buffer == BACKBUFFER_INDEX && !surface.is_null() && shot_is_pending() {
        let surface_address = surface as usize;
        let _ = panic::catch_unwind(|| capture_from_surface(surface_address));
    }
    result
}

#[expect(
    unsafe_code,
    reason = "borrows the texture the original GetBuffer just returned"
)]
fn capture_from_surface(surface_address: usize) {
    // SAFETY: the address is the out pointer the game's caller passed to GetBuffer, which the
    // original just filled with an interface pointer.
    let raw = unsafe { *(surface_address as *const *mut c_void) };
    // SAFETY: a non-null pointer there is a live COM object that the caller owns.
    if let Some(texture) = unsafe { ID3D11Texture2D::from_raw_borrowed(&raw) } {
        capture_if_requested(texture);
    }
}

#[expect(
    unsafe_code,
    reason = "creates a throwaway swap chain to read the method address"
)]
fn find_get_buffer_address() -> Option<usize> {
    let dummy = DummyHwnd::new();
    let mut device: Option<ID3D11Device> = None;
    let mut context: Option<ID3D11DeviceContext> = None;
    let mut swap_chain: Option<IDXGISwapChain> = None;
    let desc = DXGI_SWAP_CHAIN_DESC {
        BufferDesc: DXGI_MODE_DESC {
            Format: DXGI_FORMAT_R8G8B8A8_UNORM,
            ScanlineOrdering: DXGI_MODE_SCANLINE_ORDER_UNSPECIFIED,
            Scaling: DXGI_MODE_SCALING_UNSPECIFIED,
            ..Default::default()
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 1,
        OutputWindow: dummy.hwnd(),
        Windowed: BOOL(1),
        SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            ..Default::default()
        },
        ..Default::default()
    };
    // SAFETY: every pointer passed is valid for the call, as in hudhook's own probe.
    unsafe {
        D3D11CreateDeviceAndSwapChain(
            None,
            D3D_DRIVER_TYPE_HARDWARE,
            HMODULE(ptr::null_mut()),
            D3D11_CREATE_DEVICE_FLAG(0),
            Some(&[D3D_FEATURE_LEVEL_10_0, D3D_FEATURE_LEVEL_11_0]),
            D3D11_SDK_VERSION,
            Some(&desc),
            Some(&raw mut swap_chain),
            Some(&raw mut device),
            None,
            Some(&raw mut context),
        )
        .ok()?;
    }
    let swap_chain = swap_chain?;
    Some(swap_chain.vtable().GetBuffer as usize)
}

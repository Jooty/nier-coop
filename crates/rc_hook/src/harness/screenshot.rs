use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Sender};
use std::sync::{Mutex, PoisonError};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hudhook::windows::Win32::Graphics::Direct3D11::{
    D3D11_CPU_ACCESS_READ, D3D11_MAP_READ, D3D11_MAPPED_SUBRESOURCE, D3D11_TEXTURE2D_DESC,
    D3D11_USAGE_STAGING, ID3D11Texture2D,
};
use rc_game::harness::{BackbufferFormat, pixels_to_rgb};
use serde_json::{Value, json};

use crate::bridge::error_reply;

const SHOTS_DIRECTORY: &str = "replicant-coop-shots";
const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_DOWNSCALE: u64 = 8;

static PENDING_SHOT: Mutex<Option<ShotRequest>> = Mutex::new(None);

struct ShotRequest {
    path: PathBuf,
    downscale: usize,
    reply: Sender<Result<ShotInfo, String>>,
}

struct ShotInfo {
    width: usize,
    height: usize,
    bytes: u64,
}

pub(super) fn screenshot_reply(request: &Value) -> Value {
    if request.get("include_overlay").and_then(Value::as_bool) == Some(true) {
        return error_reply(
            "include_overlay is not supported yet: shots are taken before the overlay draws",
        );
    }
    let downscale = request
        .get("downscale")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, MAX_DOWNSCALE) as usize;
    let path = request
        .get("path")
        .and_then(Value::as_str)
        .map_or_else(default_path, PathBuf::from);
    let (sender, receiver) = mpsc::channel();
    *PENDING_SHOT.lock().unwrap_or_else(PoisonError::into_inner) = Some(ShotRequest {
        path: path.clone(),
        downscale,
        reply: sender,
    });
    match receiver.recv_timeout(CAPTURE_TIMEOUT) {
        Ok(Ok(info)) => json!({
            "ok": true,
            "path": path.display().to_string(),
            "width": info.width,
            "height": info.height,
            "bytes": info.bytes,
        }),
        Ok(Err(message)) => error_reply(&message),
        Err(_) => {
            PENDING_SHOT
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take();
            error_reply("no frame was drawn within 10 s, is the screenshot hook installed?")
        }
    }
}

pub(super) fn capture_if_requested(backbuffer: &ID3D11Texture2D) {
    let request = PENDING_SHOT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .take();
    if let Some(request) = request {
        let outcome = capture(backbuffer, &request.path, request.downscale);
        let _ = request.reply.send(outcome);
    }
}

pub(super) fn shot_is_pending() -> bool {
    PENDING_SHOT
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .is_some()
}

fn default_path() -> PathBuf {
    let milliseconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis());
    Path::new(SHOTS_DIRECTORY).join(format!("shot-{milliseconds}.png"))
}

#[expect(
    unsafe_code,
    reason = "copies the backbuffer to a CPU-readable staging texture"
)]
fn capture(
    backbuffer: &ID3D11Texture2D,
    path: &Path,
    downscale: usize,
) -> Result<ShotInfo, String> {
    let mut desc = D3D11_TEXTURE2D_DESC::default();
    // SAFETY: `desc` is a valid out pointer and `backbuffer` is a live texture.
    unsafe { backbuffer.GetDesc(&raw mut desc) };
    let format = u32::try_from(desc.Format.0)
        .ok()
        .and_then(BackbufferFormat::from_dxgi)
        .ok_or_else(|| format!("unsupported backbuffer format {}", desc.Format.0))?;
    if desc.SampleDesc.Count > 1 {
        return Err("the backbuffer is multisampled".to_owned());
    }
    desc.Usage = D3D11_USAGE_STAGING;
    desc.BindFlags = 0;
    desc.MiscFlags = 0;
    desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
    // SAFETY: the texture, its device and the immediate context are live for this call, and
    // this runs on the render thread that owns the context.
    let (rows, row_pitch) = unsafe {
        let device = backbuffer.GetDevice().map_err(|error| error.to_string())?;
        let context = device
            .GetImmediateContext()
            .map_err(|error| error.to_string())?;
        let mut staging: Option<ID3D11Texture2D> = None;
        device
            .CreateTexture2D(&desc, None, Some(&raw mut staging))
            .map_err(|error| error.to_string())?;
        let staging = staging.ok_or("no staging texture was created")?;
        context.CopyResource(&staging, backbuffer);
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        context
            .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&raw mut mapped))
            .map_err(|error| error.to_string())?;
        let row_pitch = mapped.RowPitch as usize;
        let length = row_pitch * desc.Height as usize;
        let rows = core::slice::from_raw_parts(mapped.pData.cast::<u8>(), length).to_vec();
        context.Unmap(&staging, 0);
        (rows, row_pitch)
    };
    let width = desc.Width as usize;
    let height = desc.Height as usize;
    let rgb = pixels_to_rgb(format, &rows, width, height, row_pitch, downscale);
    let (out_width, out_height) = (width.div_ceil(downscale), height.div_ceil(downscale));
    write_png(path, &rgb, out_width, out_height)?;
    let bytes = fs::metadata(path).map_or(0, |metadata| metadata.len());
    Ok(ShotInfo {
        width: out_width,
        height: out_height,
        bytes,
    })
}

fn write_png(path: &Path, rgb: &[u8], width: usize, height: usize) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let file = File::create(path).map_err(|error| error.to_string())?;
    let mut encoder = png::Encoder::new(BufWriter::new(file), width as u32, height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Fast);
    let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
    writer
        .write_image_data(rgb)
        .map_err(|error| error.to_string())
}

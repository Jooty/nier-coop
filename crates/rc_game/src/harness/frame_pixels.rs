const DXGI_FORMAT_R10G10B10A2_UNORM: u32 = 24;
const DXGI_FORMAT_R8G8B8A8_UNORM: u32 = 28;
const DXGI_FORMAT_R8G8B8A8_UNORM_SRGB: u32 = 29;
const DXGI_FORMAT_B8G8R8A8_UNORM: u32 = 87;
const DXGI_FORMAT_B8G8R8X8_UNORM: u32 = 88;
const DXGI_FORMAT_B8G8R8A8_UNORM_SRGB: u32 = 91;
const DXGI_FORMAT_B8G8R8X8_UNORM_SRGB: u32 = 93;
const BYTES_PER_PIXEL: usize = 4;
const RGB_BYTES_PER_PIXEL: usize = 3;
const TEN_BIT_MASK: u32 = 0x3FF;
const TEN_BIT_TO_EIGHT_BIT_SHIFT: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackbufferFormat {
    Bgra8,
    Rgba8,
    Rgb10A2,
}

impl BackbufferFormat {
    pub fn from_dxgi(format: u32) -> Option<Self> {
        match format {
            DXGI_FORMAT_B8G8R8A8_UNORM
            | DXGI_FORMAT_B8G8R8X8_UNORM
            | DXGI_FORMAT_B8G8R8A8_UNORM_SRGB
            | DXGI_FORMAT_B8G8R8X8_UNORM_SRGB => Some(Self::Bgra8),
            DXGI_FORMAT_R8G8B8A8_UNORM | DXGI_FORMAT_R8G8B8A8_UNORM_SRGB => Some(Self::Rgba8),
            DXGI_FORMAT_R10G10B10A2_UNORM => Some(Self::Rgb10A2),
            _ => None,
        }
    }
}

pub fn pixels_to_rgb(
    format: BackbufferFormat,
    rows: &[u8],
    width: usize,
    height: usize,
    row_pitch: usize,
    downscale: usize,
) -> Vec<u8> {
    let step = downscale.max(1);
    let mut rgb =
        Vec::with_capacity(width.div_ceil(step) * height.div_ceil(step) * RGB_BYTES_PER_PIXEL);
    for y in (0..height).step_by(step) {
        for x in (0..width).step_by(step) {
            let at = y * row_pitch + x * BYTES_PER_PIXEL;
            let Some(pixel) = rows.get(at..at + BYTES_PER_PIXEL) else {
                rgb.extend_from_slice(&[0; RGB_BYTES_PER_PIXEL]);
                continue;
            };
            rgb.extend_from_slice(&pixel_to_rgb(
                format,
                [pixel[0], pixel[1], pixel[2], pixel[3]],
            ));
        }
    }
    rgb
}

fn pixel_to_rgb(format: BackbufferFormat, pixel: [u8; 4]) -> [u8; 3] {
    match format {
        BackbufferFormat::Bgra8 => [pixel[2], pixel[1], pixel[0]],
        BackbufferFormat::Rgba8 => [pixel[0], pixel[1], pixel[2]],
        BackbufferFormat::Rgb10A2 => {
            let packed = u32::from_le_bytes(pixel);
            let channel = |shift: u32| {
                u8::try_from(((packed >> shift) & TEN_BIT_MASK) >> TEN_BIT_TO_EIGHT_BIT_SHIFT)
                    .unwrap_or(u8::MAX)
            };
            [channel(0), channel(10), channel(20)]
        }
    }
}

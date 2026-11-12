use anyhow::{bail, Context, Result};
use std::path::Path;
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, ID2D1DeviceContext, D2D1_BITMAP_PROPERTIES1, D2D1_BITMAP_OPTIONS_NONE,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_ContainerFormatGif, GUID_WICPixelFormat32bppPBGRA,
    IWICBitmapDecoder, IWICBitmapFrameDecode, IWICBitmapSource, IWICImagingFactory,
    IWICMetadataQueryReader, WICBitmapDitherTypeNone, WICBitmapPaletteTypeCustom,
    WICDecodeMetadataCacheOnDemand, WICDecodeMetadataCacheOnLoad,
};
use windows::Win32::UI::Shell::SHCreateMemStream;
use windows::Win32::System::Com::StructuredStorage::{PropVariantClear, PROPVARIANT};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};
use windows::Win32::System::Variant::{VT_UI1, VT_UI2, VT_UI4};
use windows::Win32::UI::WindowsAndMessaging::HICON;

const DEFAULT_FRAME_DELAY_MS: u32 = 100;
const MAX_DIMENSION_PX: u32 = 480;

/// One composited animation frame in premultiplied BGRA8 format.
#[derive(Clone)]
pub struct DecodedFrame {
    pub width: u32,
    pub height: u32,
    pub delay_ms: u32,
    pub pixels_pbgra: Vec<u8>,
}

/// Decoded image or animated GIF ready to upload to GPU `ID2D1Bitmap1` textures.
pub struct DecodedAnimation {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<DecodedFrame>,
}

/// GPU-resident animated bitmap sequence backed by `ID2D1Bitmap1`.
pub struct GpuAnimation {
    pub width: u32,
    pub height: u32,
    pub frames: Vec<GpuFrame>,
}

pub struct GpuFrame {
    pub delay_ms: u32,
    pub bitmap: ID2D1Bitmap1,
}

impl GpuAnimation {
    fn upload_frames(dc: &ID2D1DeviceContext, decoded: &DecodedAnimation) -> Result<Vec<GpuFrame>> {
        let props = D2D1_BITMAP_PROPERTIES1 {
            pixelFormat: D2D1_PIXEL_FORMAT {
                format: DXGI_FORMAT_B8G8R8A8_UNORM,
                alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
            },
            dpiX: 96.0,
            dpiY: 96.0,
            bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
            colorContext: Default::default(),
        };

        let mut gpu_frames = Vec::with_capacity(decoded.frames.len());
        for frame in &decoded.frames {
            let size = D2D_SIZE_U {
                width: frame.width,
                height: frame.height,
            };
            let pitch = frame.width * 4;
            let bmp = unsafe {
                dc.CreateBitmap(
                    size,
                    Some(frame.pixels_pbgra.as_ptr() as *const _),
                    pitch,
                    &props,
                )
            }
            .context("failed to upload decoded frame to ID2D1Bitmap1")?;

            gpu_frames.push(GpuFrame {
                delay_ms: frame.delay_ms,
                bitmap: bmp,
            });
        }
        Ok(gpu_frames)
    }

    pub fn from_decoded(dc: &ID2D1DeviceContext, decoded: &DecodedAnimation) -> Result<Self> {
        let frames = Self::upload_frames(dc, decoded)?;
        Ok(Self {
            width: decoded.width,
            height: decoded.height,
            frames,
        })
    }

    /// Appends additional decoded frames onto an already-uploaded first-frame preview.
    pub fn append_decoded(&mut self, dc: &ID2D1DeviceContext, tail: &DecodedAnimation) -> Result<()> {
        let mut more = Self::upload_frames(dc, tail)?;
        self.frames.append(&mut more);
        Ok(())
    }

    pub fn is_animated(&self) -> bool {
        self.frames.len() > 1
    }
}

/// Decodes a local `.gif`, `.webp`, `.png`, or `.jpg` into premultiplied BGRA frames via WIC.
pub fn decode_image_file(path: &Path) -> Result<DecodedAnimation> {
    decode_image_file_progressive(path, None::<fn(DecodedAnimation)>)
}

/// Decodes a local image file, optionally emitting Frame 0 immediately for multi-frame GIFs
/// before decoding the remaining frames (`tail` contains frames `1..N` when callback fires).
pub fn decode_image_file_progressive<F>(
    path: &Path,
    on_first_frame: Option<F>,
) -> Result<DecodedAnimation>
where
    F: FnMut(DecodedAnimation),
{
    let factory: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .context("failed to create IWICImagingFactory")?;

    let wide_path = HSTRING::from(path.as_os_str());
    let cache_mode = if on_first_frame.is_some() {
        WICDecodeMetadataCacheOnDemand
    } else {
        WICDecodeMetadataCacheOnLoad
    };
    let decoder: IWICBitmapDecoder = unsafe {
        factory.CreateDecoderFromFilename(&wide_path, None, GENERIC_READ, cache_mode)
    }
    .with_context(|| format!("failed to decode image {}", path.display()))?;

    decode_from_decoder_with_hook(&factory, &decoder, &path.display().to_string(), on_first_frame)
}

/// Decodes in-memory image bytes (`.gif`, `.webp`, `.png`, `.jpg`) into premultiplied BGRA frames via WIC.
pub fn decode_image_bytes(bytes: &[u8]) -> Result<DecodedAnimation> {
    if bytes.is_empty() {
        bail!("image byte buffer is empty");
    }

    let factory: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .context("failed to create IWICImagingFactory")?;

    let stream = unsafe { SHCreateMemStream(Some(bytes)) }
        .context("SHCreateMemStream returned null for image bytes")?;

    let decoder: IWICBitmapDecoder = unsafe {
        factory.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnLoad)
    }
    .context("failed to decode in-memory image stream")?;

    decode_from_decoder_with_hook(&factory, &decoder, "<memory>", None::<fn(DecodedAnimation)>)
}

fn decode_from_decoder_with_hook<F>(
    factory: &IWICImagingFactory,
    decoder: &IWICBitmapDecoder,
    label: &str,
    on_first_frame: Option<F>,
) -> Result<DecodedAnimation>
where
    F: FnMut(DecodedAnimation),
{
    let frame_count = unsafe { decoder.GetFrameCount() }.context("GetFrameCount failed")?;
    if frame_count == 0 {
        bail!("image contains zero frames: {label}");
    }

    let container_guid = unsafe { decoder.GetContainerFormat() }.unwrap_or_default();
    let is_gif = container_guid == GUID_ContainerFormatGif || frame_count > 1;

    if !is_gif || frame_count == 1 {
        let frame = unsafe { decoder.GetFrame(0) }.context("GetFrame(0) failed")?;
        let (raw_w, raw_h, raw_pixels) = convert_frame_to_pbgra(factory, &frame)?;
        let (w, h, pixels) = downscale_if_needed(raw_w, raw_h, &raw_pixels, MAX_DIMENSION_PX);
        return Ok(DecodedAnimation {
            width: w,
            height: h,
            frames: vec![DecodedFrame {
                width: w,
                height: h,
                delay_ms: DEFAULT_FRAME_DELAY_MS,
                pixels_pbgra: pixels,
            }],
        });
    }

    decode_gif_frames(factory, decoder, frame_count, on_first_frame)
}

fn decode_gif_frames<F>(
    factory: &IWICImagingFactory,
    decoder: &IWICBitmapDecoder,
    frame_count: u32,
    mut on_first_frame: Option<F>,
) -> Result<DecodedAnimation>
where
    F: FnMut(DecodedAnimation),
{
    let first_frame = unsafe { decoder.GetFrame(0) }.context("GetFrame(0) failed")?;
    let mut canvas_w = 0u32;
    let mut canvas_h = 0u32;
    unsafe { first_frame.GetSize(&mut canvas_w, &mut canvas_h) }?;

    if let Ok(global_meta) = unsafe { decoder.GetMetadataQueryReader() } {
        if let Some(w) = read_meta_u32(&global_meta, "/logscrdesc/Width") {
            if w > 0 {
                canvas_w = w;
            }
        }
        if let Some(h) = read_meta_u32(&global_meta, "/logscrdesc/Height") {
            if h > 0 {
                canvas_h = h;
            }
        }
    }

    if canvas_w == 0 || canvas_h == 0 {
        bail!("invalid GIF dimensions {canvas_w}x{canvas_h}");
    }

    let canvas_len = (canvas_w as usize)
        .checked_mul(canvas_h as usize)
        .and_then(|n| n.checked_mul(4))
        .context("GIF logical canvas size overflow")?;

    let mut canvas = vec![0u8; canvas_len];
    let mut prev_canvas = vec![0u8; canvas_len];
    let mut frames = Vec::with_capacity(frame_count as usize);
    let plan = DownscalePlan::new(canvas_w, canvas_h, MAX_DIMENSION_PX);
    let out_w = plan.nw;
    let out_h = plan.nh;

    for idx in 0..frame_count {
        let frame = unsafe { decoder.GetFrame(idx) }
            .with_context(|| format!("GetFrame({idx}) failed"))?;
        let (fw, fh, fpixels) = convert_frame_to_pbgra(factory, &frame)?;

        let mut left = 0u32;
        let mut top = 0u32;
        let mut delay_ms = DEFAULT_FRAME_DELAY_MS;
        let mut disposal = 0u32;

        if let Ok(meta) = unsafe { frame.GetMetadataQueryReader() } {
            left = read_meta_u32(&meta, "/imgdesc/Left").unwrap_or(0);
            top = read_meta_u32(&meta, "/imgdesc/Top").unwrap_or(0);
            if let Some(raw_delay) = read_meta_u32(&meta, "/grctlext/Delay") {
                let ms = raw_delay.saturating_mul(10);
                delay_ms = if ms < 20 { DEFAULT_FRAME_DELAY_MS } else { ms };
            }
            disposal = read_meta_u32(&meta, "/grctlext/Disposal").unwrap_or(0);
        }

        if disposal == 3 {
            prev_canvas.copy_from_slice(&canvas);
        }

        composite_pbgra(&mut canvas, canvas_w, canvas_h, &fpixels, fw, fh, left, top);

        let scaled = plan.apply(&canvas);
        let decoded_frame = DecodedFrame {
            width: out_w,
            height: out_h,
            delay_ms,
            pixels_pbgra: scaled,
        };
        if idx == 0 {
            if let Some(ref mut cb) = on_first_frame {
                cb(DecodedAnimation {
                    width: out_w,
                    height: out_h,
                    frames: vec![decoded_frame],
                });
            } else {
                frames.push(decoded_frame);
            }
        } else {
            frames.push(decoded_frame);
        }

        match disposal {
            2 => clear_rect_pbgra(&mut canvas, canvas_w, canvas_h, left, top, fw, fh),
            3 => canvas.copy_from_slice(&prev_canvas),
            _ => {}
        }
    }

    Ok(DecodedAnimation {
        width: out_w,
        height: out_h,
        frames,
    })
}

/// Decodes a Win32 `HICON` handle directly into premultiplied BGRA8 via WIC.
pub fn decode_hicon_to_frame(hicon_raw: isize) -> Result<DecodedFrame> {
    if hicon_raw == 0 {
        bail!("null HICON");
    }
    let factory: IWICImagingFactory =
        unsafe { CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER) }
            .context("failed to create WIC imaging factory for HICON")?;
    let wic_bmp = unsafe { factory.CreateBitmapFromHICON(HICON(hicon_raw as *mut _)) }
        .context("CreateBitmapFromHICON failed")?;
    let source: IWICBitmapSource = wic_bmp.cast()?;
    let (width, height, pixels_pbgra) = convert_source_to_pbgra(&factory, &source)?;
    Ok(DecodedFrame {
        width,
        height,
        delay_ms: 0,
        pixels_pbgra,
    })
}

fn convert_frame_to_pbgra(
    factory: &IWICImagingFactory,
    frame: &IWICBitmapFrameDecode,
) -> Result<(u32, u32, Vec<u8>)> {
    let source: IWICBitmapSource = frame.cast()?;
    convert_source_to_pbgra(factory, &source)
}

fn convert_source_to_pbgra(
    factory: &IWICImagingFactory,
    source: &IWICBitmapSource,
) -> Result<(u32, u32, Vec<u8>)> {
    let mut width = 0u32;
    let mut height = 0u32;
    unsafe { source.GetSize(&mut width, &mut height) }?;

    let converter = unsafe { factory.CreateFormatConverter() }?;
    unsafe {
        converter.Initialize(
            source,
            &GUID_WICPixelFormat32bppPBGRA,
            WICBitmapDitherTypeNone,
            None,
            0.0,
            WICBitmapPaletteTypeCustom,
        )?;
    }

    let stride = width.checked_mul(4).context("stride overflow")?;
    let buf_len = (stride as usize)
        .checked_mul(height as usize)
        .context("pixel buffer overflow")?;
    let mut buffer = vec![0u8; buf_len];
    unsafe { converter.CopyPixels(std::ptr::null(), stride, &mut buffer) }?;

    Ok((width, height, buffer))
}

fn read_meta_u32(reader: &IWICMetadataQueryReader, query: &str) -> Option<u32> {
    let wide = HSTRING::from(query);
    let mut var = PROPVARIANT::default();
    let res = unsafe { reader.GetMetadataByName(&wide, &mut var) };
    if res.is_err() {
        return None;
    }

    let val = unsafe {
        let inner = &var.Anonymous.Anonymous;
        match inner.vt {
            VT_UI1 => Some(inner.Anonymous.bVal as u32),
            VT_UI2 => Some(inner.Anonymous.uiVal as u32),
            VT_UI4 => Some(inner.Anonymous.ulVal),
            _ => None,
        }
    };

    unsafe {
        let _ = PropVariantClear(&mut var);
    }
    val
}

fn composite_pbgra(
    dst: &mut [u8],
    dst_w: u32,
    dst_h: u32,
    src: &[u8],
    src_w: u32,
    src_h: u32,
    left: u32,
    top: u32,
) {
    let max_x = dst_w.min(left.saturating_add(src_w));
    let max_y = dst_h.min(top.saturating_add(src_h));

    for dy in top..max_y {
        let sy = dy - top;
        for dx in left..max_x {
            let sx = dx - left;
            let si = ((sy * src_w + sx) * 4) as usize;
            let di = ((dy * dst_w + dx) * 4) as usize;

            let sa = src[si + 3] as u32;
            if sa == 0 {
                continue;
            }
            if sa == 255 {
                dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
                continue;
            }

            let inv_a = 255 - sa;
            for c in 0..4 {
                let s = src[si + c] as u32;
                let d = dst[di + c] as u32;
                dst[di + c] = (s + (d * inv_a + 127) / 255).min(255) as u8;
            }
        }
    }
}

fn clear_rect_pbgra(
    buf: &mut [u8],
    buf_w: u32,
    buf_h: u32,
    left: u32,
    top: u32,
    w: u32,
    h: u32,
) {
    let max_x = buf_w.min(left.saturating_add(w));
    let max_y = buf_h.min(top.saturating_add(h));
    for y in top..max_y {
        let start = ((y * buf_w + left) * 4) as usize;
        let end = ((y * buf_w + max_x) * 4) as usize;
        if end <= buf.len() && start < end {
            buf[start..end].fill(0);
        }
    }
}

struct DownscalePlan {
    w: u32,
    h: u32,
    nw: u32,
    nh: u32,
    bypass: bool,
    x_map: Vec<(usize, usize, u32, u32)>,
    y_map: Vec<(usize, usize, u32, u32)>,
}

impl DownscalePlan {
    fn new(w: u32, h: u32, max_dim: u32) -> Self {
        if w <= max_dim && h <= max_dim {
            return Self {
                w,
                h,
                nw: w,
                nh: h,
                bypass: true,
                x_map: Vec::new(),
                y_map: Vec::new(),
            };
        }
        let scale = (max_dim as f32 / w.max(h) as f32).min(1.0);
        let nw = ((w as f32 * scale).round() as u32).max(1);
        let nh = ((h as f32 * scale).round() as u32).max(1);

        let build_axis = |src_len: u32, dst_len: u32, stride: usize| -> Vec<(usize, usize, u32, u32)> {
            (0..dst_len)
                .map(|n| {
                    let s = ((n as f32 + 0.5) * src_len as f32 / dst_len as f32 - 0.5)
                        .clamp(0.0, (src_len - 1) as f32);
                    let p0 = s.floor() as u32;
                    let p1 = (p0 + 1).min(src_len - 1);
                    let f = ((s - p0 as f32) * 256.0).round() as u32;
                    let w1 = f.min(256);
                    let w0 = 256 - w1;
                    (p0 as usize * stride, p1 as usize * stride, w0, w1)
                })
                .collect()
        };

        let x_map = build_axis(w, nw, 4);
        let y_map = build_axis(h, nh, (w * 4) as usize);
        Self {
            w,
            h,
            nw,
            nh,
            bypass: false,
            x_map,
            y_map,
        }
    }

    fn apply(&self, src: &[u8]) -> Vec<u8> {
        if self.bypass {
            return src.to_vec();
        }
        let _ = (self.w, self.h);
        let mut out = vec![0u8; (self.nw * self.nh * 4) as usize];
        let mut di = 0usize;
        for &(row0, row1, wy0, wy1) in &self.y_map {
            for &(col0, col1, wx0, wx1) in &self.x_map {
                let i00 = row0 + col0;
                let i10 = row0 + col1;
                let i01 = row1 + col0;
                let i11 = row1 + col1;

                let w00 = wx0 * wy0;
                let w10 = wx1 * wy0;
                let w01 = wx0 * wy1;
                let w11 = wx1 * wy1;

                for c in 0..4 {
                    let v = src[i00 + c] as u32 * w00
                        + src[i10 + c] as u32 * w10
                        + src[i01 + c] as u32 * w01
                        + src[i11 + c] as u32 * w11;
                    out[di + c] = ((v + 32768) >> 16).min(255) as u8;
                }
                di += 4;
            }
        }
        out
    }
}

fn downscale_if_needed(w: u32, h: u32, src: &[u8], max_dim: u32) -> (u32, u32, Vec<u8>) {
    let plan = DownscalePlan::new(w, h, max_dim);
    (plan.nw, plan.nh, plan.apply(src))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use windows::Win32::System::Com::{CoInitializeEx, CoUninitialize, COINIT_APARTMENTTHREADED};

    fn minimal_two_frame_gif_bytes() -> &'static [u8] {
        #[rustfmt::skip]
        const GIF_BYTES: &[u8] = &[
            b'G', b'I', b'F', b'8', b'9', b'a',
            0x01, 0x00, 0x01, 0x00, 0x80, 0x00, 0x00, // 1x1, 2-color GCT
            0xFF, 0x00, 0x00, 0x00, 0x00, 0xFF,       // Red, Blue
            0x21, 0xFF, 0x0B, b'N', b'E', b'T', b'S', b'C', b'A', b'P', b'E', b'2', b'.', b'0',
            0x03, 0x01, 0x00, 0x00, 0x00,             // Loop forever
            // Frame 1: delay 10 (100ms)
            0x21, 0xF9, 0x04, 0x04, 0x0A, 0x00, 0x00, 0x00,
            0x2C, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            0x02, 0x02, 0x44, 0x01, 0x00,
            // Frame 2: delay 20 (200ms)
            0x21, 0xF9, 0x04, 0x04, 0x14, 0x00, 0x00, 0x00,
            0x2C, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00,
            0x02, 0x02, 0x4C, 0x01, 0x00,
            0x3B, // Trailer
        ];
        GIF_BYTES
    }

    #[test]
    fn wic_decodes_minimal_two_frame_gif_with_delays() {
        let tmp = std::env::temp_dir().join("focusgrow_gfx_test_2frame.gif");
        fs::write(&tmp, minimal_two_frame_gif_bytes()).unwrap();

        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let decoded = decode_image_file(&tmp).expect("decode 2-frame gif");
        unsafe {
            CoUninitialize();
        }
        let _ = fs::remove_file(&tmp);

        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.frames.len(), 2);
        assert_eq!(decoded.frames[0].delay_ms, 100);
        assert_eq!(decoded.frames[1].delay_ms, 200);
    }

    #[test]
    fn wic_decodes_two_frame_gif_from_memory_bytes() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let decoded = decode_image_bytes(minimal_two_frame_gif_bytes()).expect("decode gif bytes");
        unsafe {
            CoUninitialize();
        }

        assert_eq!(decoded.width, 1);
        assert_eq!(decoded.height, 1);
        assert_eq!(decoded.frames.len(), 2);
        assert_eq!(decoded.frames[0].delay_ms, 100);
        assert_eq!(decoded.frames[1].delay_ms, 200);
    }

    #[test]
    fn downscale_preserves_dimensions_when_under_cap() {
        let px = vec![255u8; 4 * 4 * 4];
        let (w, h, out) = downscale_if_needed(4, 4, &px, 720);
        assert_eq!((w, h), (4, 4));
        assert_eq!(out.len(), px.len());
    }
}


use anyhow::{Context, Result};
use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_COLOR_F, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, ID2D1Bitmap1, ID2D1Device, ID2D1DeviceContext, ID2D1Factory1,
    ID2D1SolidColorBrush, D2D1_BITMAP_OPTIONS_CANNOT_DRAW, D2D1_BITMAP_OPTIONS_TARGET,
    D2D1_BITMAP_PROPERTIES1, D2D1_DEVICE_CONTEXT_OPTIONS_NONE, D2D1_FACTORY_TYPE_SINGLE_THREADED,
};
use windows::Win32::Graphics::Direct3D::{
    D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP, D3D_FEATURE_LEVEL_10_0,
    D3D_FEATURE_LEVEL_10_1, D3D_FEATURE_LEVEL_11_0, D3D_FEATURE_LEVEL_11_1,
};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::DirectComposition::{
    DCompositionCreateDevice, IDCompositionDevice, IDCompositionTarget, IDCompositionVisual,
};
use windows::Win32::Graphics::DirectWrite::{
    DWriteCreateFactory, IDWriteFactory, IDWriteTextFormat, DWRITE_FACTORY_TYPE_SHARED,
    DWRITE_FONT_STRETCH_NORMAL, DWRITE_FONT_STYLE_NORMAL, DWRITE_FONT_WEIGHT,
    DWRITE_PARAGRAPH_ALIGNMENT, DWRITE_TEXT_ALIGNMENT, DWRITE_TRIMMING,
    DWRITE_TRIMMING_GRANULARITY_CHARACTER, DWRITE_WORD_WRAPPING_NO_WRAP,
};
use windows::Win32::Graphics::Dxgi::Common::{
    DXGI_ALPHA_MODE_PREMULTIPLIED, DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_FORMAT_UNKNOWN,
    DXGI_SAMPLE_DESC,
};
use windows::Win32::Graphics::Dxgi::{
    CreateDXGIFactory2, IDXGIDevice, IDXGIFactory2, IDXGISurface, IDXGISwapChain1,
    DXGI_CREATE_FACTORY_FLAGS, DXGI_PRESENT, DXGI_SCALING_STRETCH, DXGI_SWAP_CHAIN_DESC1,
    DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL, DXGI_USAGE_RENDER_TARGET_OUTPUT,
};

/// Shared Direct3D11 / Direct2D1 / DirectWrite / DirectComposition GPU context for a window.
pub struct DCompSurface {
    pub d3d_device: ID3D11Device,
    pub d2d_factory: ID2D1Factory1,
    pub d2d_device: ID2D1Device,
    pub d2d_context: ID2D1DeviceContext,
    pub dwrite_factory: IDWriteFactory,
    pub dcomp_device: IDCompositionDevice,
    pub dcomp_target: IDCompositionTarget,
    pub dcomp_visual: IDCompositionVisual,
    pub swap_chain: IDXGISwapChain1,
    pub target_bitmap: Option<ID2D1Bitmap1>,
    pub width_px: u32,
    pub height_px: u32,
    pub dpi: f32,
}

impl DCompSurface {
    pub fn new(hwnd: HWND, width_px: u32, height_px: u32, dpi: f32) -> Result<Self> {
        let width_px = width_px.max(1);
        let height_px = height_px.max(1);
        let dpi = dpi.max(96.0);

        let d3d_device = create_d3d11_device()?;
        let dxgi_device: IDXGIDevice = d3d_device.cast()?;

        let d2d_factory: ID2D1Factory1 =
            unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, None) }
                .context("D2D1CreateFactory failed")?;

        let d2d_device = unsafe { d2d_factory.CreateDevice(&dxgi_device) }
            .context("ID2D1Factory1::CreateDevice failed")?;

        let d2d_context = unsafe { d2d_device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE) }
            .context("ID2D1Device::CreateDeviceContext failed")?;

        let dwrite_factory: IDWriteFactory =
            unsafe { DWriteCreateFactory(DWRITE_FACTORY_TYPE_SHARED) }
                .context("DWriteCreateFactory failed")?;

        let dxgi_factory: IDXGIFactory2 =
            unsafe { CreateDXGIFactory2(DXGI_CREATE_FACTORY_FLAGS(0)) }
                .context("CreateDXGIFactory2 failed")?;

        let swap_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width_px,
            Height: height_px,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            Stereo: false.into(),
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
            AlphaMode: DXGI_ALPHA_MODE_PREMULTIPLIED,
            Flags: 0,
        };

        let swap_chain = unsafe {
            dxgi_factory.CreateSwapChainForComposition(&d3d_device, &swap_desc, None)
        }
        .context("CreateSwapChainForComposition failed")?;

        let dcomp_device: IDCompositionDevice = unsafe { DCompositionCreateDevice(&dxgi_device) }
            .context("DCompositionCreateDevice failed")?;
        let dcomp_target = unsafe { dcomp_device.CreateTargetForHwnd(hwnd, true) }
            .context("IDCompositionDevice::CreateTargetForHwnd failed")?;
        let dcomp_visual =
            unsafe { dcomp_device.CreateVisual() }.context("CreateVisual failed")?;

        unsafe {
            dcomp_visual.SetContent(&swap_chain)?;
            dcomp_target.SetRoot(&dcomp_visual)?;
            dcomp_device.Commit()?;
        }

        let target_bitmap = bind_swapchain_target(&d2d_context, &swap_chain, dpi)?;

        Ok(Self {
            d3d_device,
            d2d_factory,
            d2d_device,
            d2d_context,
            dwrite_factory,
            dcomp_device,
            dcomp_target,
            dcomp_visual,
            swap_chain,
            target_bitmap: Some(target_bitmap),
            width_px,
            height_px,
            dpi,
        })
    }

    pub fn dip_size(&self) -> (f32, f32) {
        let scale = 96.0 / self.dpi.max(96.0);
        (self.width_px as f32 * scale, self.height_px as f32 * scale)
    }

    pub fn resize(&mut self, width_px: u32, height_px: u32, dpi: f32) -> Result<()> {
        let width_px = width_px.max(1);
        let height_px = height_px.max(1);
        let dpi = dpi.max(96.0);

        let size_changed = self.width_px != width_px || self.height_px != height_px;
        let dpi_changed = (self.dpi - dpi).abs() >= 0.01;
        if !size_changed && !dpi_changed {
            return Ok(());
        }

        if size_changed {
            unsafe {
                self.d2d_context.SetTarget(None);
            }
            self.target_bitmap = None;
            unsafe {
                self.swap_chain
                    .ResizeBuffers(0, width_px, height_px, DXGI_FORMAT_UNKNOWN, Default::default())
                    .context("IDXGISwapChain::ResizeBuffers failed")?;
            }
            let bmp = bind_swapchain_target(&self.d2d_context, &self.swap_chain, dpi)?;
            self.target_bitmap = Some(bmp);
            self.width_px = width_px;
            self.height_px = height_px;
        } else if dpi_changed {
            unsafe {
                self.d2d_context.SetDpi(dpi, dpi);
            }
        }

        self.dpi = dpi;
        Ok(())
    }

    pub fn render<F>(&self, draw_fn: F) -> Result<()>
    where
        F: FnOnce(&ID2D1DeviceContext, &ID2D1Factory1, &IDWriteFactory, f32, f32) -> Result<()>,
    {
        let (w_dip, h_dip) = self.dip_size();
        unsafe {
            self.d2d_context.BeginDraw();
            self.d2d_context.Clear(Some(&D2D1_COLOR_F {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.0,
            }));
        }

        let draw_res = draw_fn(
            &self.d2d_context,
            &self.d2d_factory,
            &self.dwrite_factory,
            w_dip,
            h_dip,
        );

        let end_res = unsafe { self.d2d_context.EndDraw(None, None) };
        draw_res?;
        end_res.context("ID2D1DeviceContext::EndDraw failed")?;

        unsafe {
            self.swap_chain
                .Present(0, DXGI_PRESENT(0))
                .ok()
                .context("IDXGISwapChain::Present failed")?;
            let _ = self.dcomp_device.Commit();
        }

        Ok(())
    }

    pub fn create_brush(&self, color: D2D1_COLOR_F) -> Result<ID2D1SolidColorBrush> {
        unsafe { self.d2d_context.CreateSolidColorBrush(&color, None) }
            .context("CreateSolidColorBrush failed")
    }
}

pub fn create_text_format(
    dwrite: &IDWriteFactory,
    family: &str,
    size: f32,
    weight: DWRITE_FONT_WEIGHT,
    text_align: DWRITE_TEXT_ALIGNMENT,
    para_align: DWRITE_PARAGRAPH_ALIGNMENT,
) -> Result<IDWriteTextFormat> {
    let family_h = HSTRING::from(family);
    let locale_h = HSTRING::from("en-us");
    let fmt = unsafe {
        dwrite.CreateTextFormat(
            &family_h,
            None,
            weight,
            DWRITE_FONT_STYLE_NORMAL,
            DWRITE_FONT_STRETCH_NORMAL,
            size,
            &locale_h,
        )
    }
    .context("CreateTextFormat failed")?;

    unsafe {
        fmt.SetTextAlignment(text_align)?;
        fmt.SetParagraphAlignment(para_align)?;
    }
    Ok(fmt)
}

/// Single-line DirectWrite format with no wrapping and automatic `…` ellipsis trimming.
pub fn create_text_format_single_line(
    dwrite: &IDWriteFactory,
    family: &str,
    size: f32,
    weight: DWRITE_FONT_WEIGHT,
    text_align: DWRITE_TEXT_ALIGNMENT,
    para_align: DWRITE_PARAGRAPH_ALIGNMENT,
) -> Result<IDWriteTextFormat> {
    let fmt = create_text_format(dwrite, family, size, weight, text_align, para_align)?;
    unsafe {
        fmt.SetWordWrapping(DWRITE_WORD_WRAPPING_NO_WRAP)?;
        if let Ok(sign) = dwrite.CreateEllipsisTrimmingSign(&fmt) {
            let trimming = DWRITE_TRIMMING {
                granularity: DWRITE_TRIMMING_GRANULARITY_CHARACTER,
                delimiter: 0,
                delimiterCount: 0,
            };
            let _ = fmt.SetTrimming(&trimming, &sign);
        }
    }
    Ok(fmt)
}

pub fn rgba(r: u8, g: u8, b: u8, a: f32) -> D2D1_COLOR_F {
    D2D1_COLOR_F {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a.clamp(0.0, 1.0),
    }
}

pub fn hex_color(hex: &str, alpha: f32) -> D2D1_COLOR_F {
    let clean = hex.trim().trim_start_matches('#');
    if clean.len() >= 6 {
        if let (Ok(r), Ok(g), Ok(b)) = (
            u8::from_str_radix(&clean[0..2], 16),
            u8::from_str_radix(&clean[2..4], 16),
            u8::from_str_radix(&clean[4..6], 16),
        ) {
            return rgba(r, g, b, alpha);
        }
    }
    rgba(96, 205, 255, alpha)
}

fn create_d3d11_device() -> Result<ID3D11Device> {
    let feature_levels = [
        D3D_FEATURE_LEVEL_11_1,
        D3D_FEATURE_LEVEL_11_0,
        D3D_FEATURE_LEVEL_10_1,
        D3D_FEATURE_LEVEL_10_0,
    ];

    for driver in [D3D_DRIVER_TYPE_HARDWARE, D3D_DRIVER_TYPE_WARP] {
        let mut device: Option<ID3D11Device> = None;
        let res = unsafe {
            D3D11CreateDevice(
                None,
                driver,
                Default::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                None,
            )
        };
        if res.is_ok() {
            if let Some(dev) = device {
                return Ok(dev);
            }
        }
    }

    anyhow::bail!("failed to create D3D11 device (hardware and WARP both failed)")
}

fn bind_swapchain_target(
    dc: &ID2D1DeviceContext,
    swap_chain: &IDXGISwapChain1,
    dpi: f32,
) -> Result<ID2D1Bitmap1> {
    let surface: IDXGISurface = unsafe { swap_chain.GetBuffer(0) }
        .context("IDXGISwapChain::GetBuffer(0) failed")?;

    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: dpi,
        dpiY: dpi,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: Default::default(),
    };

    let target = unsafe { dc.CreateBitmapFromDxgiSurface(&surface, Some(&props)) }
        .context("CreateBitmapFromDxgiSurface failed")?;

    unsafe {
        dc.SetTarget(&target);
        dc.SetDpi(dpi, dpi);
    }

    Ok(target)
}

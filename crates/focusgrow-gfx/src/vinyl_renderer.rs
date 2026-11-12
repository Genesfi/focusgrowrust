use anyhow::{Context, Result};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_GRADIENT_STOP, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    ID2D1Bitmap1, ID2D1DeviceContext, D2D1_BITMAP_BRUSH_PROPERTIES1, D2D1_BITMAP_OPTIONS_NONE,
    D2D1_BITMAP_PROPERTIES1, D2D1_BRUSH_PROPERTIES, D2D1_BUFFER_PRECISION_8BPC_UNORM,
    D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED, D2D1_COLOR_SPACE_SRGB, D2D1_ELLIPSE,
    D2D1_EXTEND_MODE_CLAMP, D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
    D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES,
};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_ALPHA_MODE_PREMULTIPLIED, D2D1_PIXEL_FORMAT,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows_numerics::{Matrix3x2, Vector2};

use crate::dcomp::{hex_color, rgba};
use crate::gif_decoder::DecodedFrame;

/// Uploads a single decoded premultiplied BGRA frame (e.g., album cover) as a GPU `ID2D1Bitmap1`.
pub fn upload_frame_bitmap(dc: &ID2D1DeviceContext, frame: &DecodedFrame) -> Result<ID2D1Bitmap1> {
    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_PREMULTIPLIED,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_NONE,
        ..Default::default()
    };

    let pitch = frame.width * 4;
    let size = D2D_SIZE_U {
        width: frame.width,
        height: frame.height,
    };

    unsafe {
        dc.CreateBitmap(
            size,
            Some(frame.pixels_pbgra.as_ptr() as *const _),
            pitch,
            &props,
        )
    }
    .context("CreateBitmap failed for cover bitmap")
}

/// Draws a GPU-accelerated vinyl disc with concentric grooves, a rotated center-cropped
/// album art label (at 80% disc radius matching FocusGrow's signature vinyl style),
/// and an optional glossy glass spindle hole in the center.
pub fn draw_vinyl_disc(
    dc: &ID2D1DeviceContext,
    cx: f32,
    cy: f32,
    radius: f32,
    angle_deg: f32,
    cover_bitmap: Option<&ID2D1Bitmap1>,
    show_spindle: bool,
    accent_hex: &str,
) -> Result<()> {
    if radius <= 2.0 {
        return Ok(());
    }

    let center = Vector2 { X: cx, Y: cy };
    let disc_ellipse = D2D1_ELLIPSE {
        point: center,
        radiusX: radius,
        radiusY: radius,
    };

    // Soft ambient shadow halo behind the disc edge
    let shadow_stops = [
        D2D1_GRADIENT_STOP {
            position: 0.78,
            color: rgba(0, 0, 0, 0.35),
        },
        D2D1_GRADIENT_STOP {
            position: 1.0,
            color: rgba(0, 0, 0, 0.0),
        },
    ];
    if let Ok(stop_col) = unsafe {
        dc.CreateGradientStopCollection(
            &shadow_stops,
            D2D1_COLOR_SPACE_SRGB,
            D2D1_COLOR_SPACE_SRGB,
            D2D1_BUFFER_PRECISION_8BPC_UNORM,
            D2D1_EXTEND_MODE_CLAMP,
            D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED,
        )
    } {
        let shadow_props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
            center,
            gradientOriginOffset: Vector2 { X: 0.0, Y: 0.0 },
            radiusX: radius * 1.06,
            radiusY: radius * 1.06,
        };
        if let Ok(shadow_brush) = unsafe { dc.CreateRadialGradientBrush(&shadow_props, None, &stop_col) } {
            unsafe {
                dc.FillEllipse(
                    &D2D1_ELLIPSE {
                        point: center,
                        radiusX: radius * 1.06,
                        radiusY: radius * 1.06,
                    },
                    &shadow_brush,
                );
            }
        }
    }

    // Glossy vinyl body radial gradient
    let body_stops = [
        D2D1_GRADIENT_STOP {
            position: 0.0,
            color: rgba(34, 34, 38, 1.0),
        },
        D2D1_GRADIENT_STOP {
            position: 0.82,
            color: rgba(16, 16, 20, 1.0),
        },
        D2D1_GRADIENT_STOP {
            position: 0.96,
            color: rgba(24, 24, 28, 1.0),
        },
        D2D1_GRADIENT_STOP {
            position: 1.0,
            color: rgba(8, 8, 10, 1.0),
        },
    ];
    let body_col = unsafe {
        dc.CreateGradientStopCollection(
            &body_stops,
            D2D1_COLOR_SPACE_SRGB,
            D2D1_COLOR_SPACE_SRGB,
            D2D1_BUFFER_PRECISION_8BPC_UNORM,
            D2D1_EXTEND_MODE_CLAMP,
            D2D1_COLOR_INTERPOLATION_MODE_PREMULTIPLIED,
        )
    }
    .context("CreateGradientStopCollection failed for vinyl body")?;

    let body_props = D2D1_RADIAL_GRADIENT_BRUSH_PROPERTIES {
        center,
        gradientOriginOffset: Vector2 { X: -radius * 0.18, Y: -radius * 0.18 },
        radiusX: radius,
        radiusY: radius,
    };
    let body_brush = unsafe { dc.CreateRadialGradientBrush(&body_props, None, &body_col) }
        .context("CreateRadialGradientBrush failed for vinyl body")?;
    unsafe {
        dc.FillEllipse(&disc_ellipse, &body_brush);
    }

    // Outer metallic rim highlight & subtle vinyl micro-grooves
    let rim_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.18), None) }?;
    let groove_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.075), None) }?;
    unsafe {
        dc.DrawEllipse(&disc_ellipse, &rim_brush, 1.1, None);
    }
    for factor in [0.955f32, 0.915, 0.875, 0.835] {
        let r_g = radius * factor;
        unsafe {
            dc.DrawEllipse(
                &D2D1_ELLIPSE {
                    point: center,
                    radiusX: r_g,
                    radiusY: r_g,
                },
                &groove_brush,
                0.85,
                None,
            );
        }
    }

    // Album art label at 80% radius (matches PipVinylOverlay.hpp proportion)
    let label_r = radius * 0.80;
    let label_ellipse = D2D1_ELLIPSE {
        point: center,
        radiusX: label_r,
        radiusY: label_r,
    };

    if let Some(bmp) = cover_bitmap {
        let bmp_size = unsafe { bmp.GetSize() };
        let min_dim = bmp_size.width.min(bmp_size.height).max(1.0);
        let scale = (label_r * 2.0) / min_dim;

        // Center-crop cover into label circle, then rotate around disc center
        let brush_transform = Matrix3x2::translation(-bmp_size.width * 0.5, -bmp_size.height * 0.5)
            * Matrix3x2::scale(scale, scale)
            * Matrix3x2::translation(cx, cy);

        let bmp_props = D2D1_BITMAP_BRUSH_PROPERTIES1 {
            extendModeX: D2D1_EXTEND_MODE_CLAMP,
            extendModeY: D2D1_EXTEND_MODE_CLAMP,
            interpolationMode: D2D1_INTERPOLATION_MODE_HIGH_QUALITY_CUBIC,
        };
        let brush_props = D2D1_BRUSH_PROPERTIES {
            opacity: 1.0,
            transform: brush_transform,
        };

        let bmp_brush = unsafe {
            dc.CreateBitmapBrush(bmp, Some(&bmp_props), Some(&brush_props))
        }
        .context("CreateBitmapBrush failed for vinyl label")?;

        unsafe {
            dc.SetTransform(&Matrix3x2::rotation_around(angle_deg, center));
            dc.FillEllipse(&label_ellipse, &bmp_brush);
            dc.SetTransform(&Matrix3x2::identity());
        }
    } else {
        let fallback_label = unsafe { dc.CreateSolidColorBrush(&hex_color(accent_hex, 0.25), None) }?;
        unsafe {
            dc.FillEllipse(&label_ellipse, &fallback_label);
        }
    }

    // Subtle dark ring + inner specular ring around the label
    let label_border = unsafe { dc.CreateSolidColorBrush(&rgba(0, 0, 0, 0.55), None) }?;
    let label_sheen = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.14), None) }?;
    unsafe {
        dc.DrawEllipse(&label_ellipse, &label_border, 1.5, None);
        dc.DrawEllipse(
            &D2D1_ELLIPSE {
                point: center,
                radiusX: (label_r - 1.5).max(1.0),
                radiusY: (label_r - 1.5).max(1.0),
            },
            &label_sheen,
            0.8,
            None,
        );
    }

    if show_spindle {
        let spindle_r = (radius * 0.14).max(6.0);
        let glass_outer = D2D1_ELLIPSE {
            point: center,
            radiusX: spindle_r * 1.45,
            radiusY: spindle_r * 1.45,
        };
        let glass_brush = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.22), None) }?;
        let dark_core = unsafe { dc.CreateSolidColorBrush(&rgba(18, 18, 22, 0.92), None) }?;
        let core_rim = unsafe { dc.CreateSolidColorBrush(&rgba(255, 255, 255, 0.35), None) }?;
        let hole_r = (spindle_r * 0.62).max(3.5);
        unsafe {
            dc.FillEllipse(&glass_outer, &glass_brush);
            dc.FillEllipse(
                &D2D1_ELLIPSE {
                    point: center,
                    radiusX: hole_r,
                    radiusY: hole_r,
                },
                &dark_core,
            );
            dc.DrawEllipse(
                &D2D1_ELLIPSE {
                    point: center,
                    radiusX: hole_r,
                    radiusY: hole_r,
                },
                &core_rim,
                1.0,
                None,
            );
        }
    }

    Ok(())
}


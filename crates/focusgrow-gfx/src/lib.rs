pub mod color_extract;
pub mod dcomp;
pub mod gif_decoder;
pub mod vinyl_renderer;

pub use color_extract::extract_album_accent;
pub use dcomp::DCompSurface;
pub use gif_decoder::{
    decode_hicon_to_frame, decode_image_bytes, decode_image_file, decode_image_file_progressive,
    DecodedAnimation, DecodedFrame, GpuAnimation,
};
pub use vinyl_renderer::{draw_vinyl_disc, upload_frame_bitmap};

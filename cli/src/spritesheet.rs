use anyhow::{Context, Result};
use image::{imageops, RgbaImage};

use crate::types::Frame;

/// Load a spritesheet PNG and split it into individual frames.
///
/// Frames are extracted left-to-right, top-to-bottom from the spritesheet
/// based on the given frame dimensions and count.
pub fn load_spritesheet(
    path: &str,
    frame_width: u32,
    frame_height: u32,
    frame_count: usize,
) -> Result<Vec<Frame>> {
    let img = image::open(path)
        .with_context(|| format!("Failed to open image: {}", path))?
        .to_rgba8();

    let sheet_w = img.width();
    let sheet_h = img.height();
    let cols = sheet_w / frame_width;

    let mut frames = Vec::with_capacity(frame_count);
    for i in 0..frame_count {
        let col = (i as u32) % cols;
        let row = (i as u32) / cols;
        let x = col * frame_width;
        let y = row * frame_height;

        if x + frame_width > sheet_w || y + frame_height > sheet_h {
            anyhow::bail!(
                "Frame {} at ({}, {}) exceeds image bounds ({}x{})",
                i,
                x,
                y,
                sheet_w,
                sheet_h
            );
        }

        let sub = image::imageops::crop_imm(&img, x, y, frame_width, frame_height).to_image();
        frames.push(Frame(sub));
    }

    Ok(frames)
}

/// Assemble frames into a spritesheet and save as PNG.
///
/// Frames are laid out left-to-right, wrapping at `columns`.
pub fn save_spritesheet(
    frames: &[Frame],
    path: &str,
    columns: usize,
) -> Result<()> {
    if frames.is_empty() {
        anyhow::bail!("No frames to save");
    }

    let frame_w = frames[0].width();
    let frame_h = frames[0].height();
    let columns = columns.min(frames.len());
    let rows = (frames.len() as f64 / columns as f64).ceil() as u32;

    let sheet_w = frame_w * columns as u32;
    let sheet_h = frame_h * rows;

    let mut sheet = RgbaImage::new(sheet_w, sheet_h);

    for (i, frame) in frames.iter().enumerate() {
        let col = (i % columns) as u32;
        let row = (i / columns) as u32;
        imageops::replace(&mut sheet, &frame.0, (col * frame_w) as i64, (row * frame_h) as i64);
    }

    sheet
        .save(path)
        .with_context(|| format!("Failed to save spritesheet to: {}", path))?;

    Ok(())
}

use image::codecs::png::PngEncoder;
use image::imageops::FilterType;
use image::{ExtendedColorType, ImageEncoder, RgbaImage};

use crate::error::{AppError, Result};
use crate::storage::{atomic_write, StoragePaths};

pub struct SavedImage {
    pub png: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub image_rel: String,
    pub thumb_rel: String,
    pub preview_rel: String,
}

pub fn parse_dib(data: &[u8]) -> Result<RgbaImage> {
    if data.len() < 40 {
        return Err(AppError::msg("DIB data is too small"));
    }
    let header_size = read_u32(data, 0) as usize;
    if header_size < 40 || header_size > data.len() {
        return Err(AppError::msg("DIB header size is invalid"));
    }
    let width_i = read_i32(data, 4);
    let height_i = read_i32(data, 8);
    let bpp = read_u16(data, 14);
    let compression = read_u32(data, 16);
    let clr_used = read_u32(data, 32);

    if width_i <= 0 || height_i == 0 {
        return Err(AppError::msg("DIB has invalid dimensions"));
    }
    let width = width_i as u32;
    let height = height_i.unsigned_abs();
    if width > 20_000 || height > 20_000 || (width as u64 * height as u64) > 120_000_000 {
        return Err(AppError::msg("DIB is too large to decode safely"));
    }

    // JPEG/PNG-compressed DIBs store a complete image stream after the header.
    if compression == 4 || compression == 5 {
        let payload = data
            .get(header_size..)
            .ok_or_else(|| AppError::msg("DIB payload is truncated"))?;
        return decode_limited(payload);
    }

    if !matches!(bpp, 1 | 4 | 8 | 16 | 24 | 32) {
        return Err(AppError::msg(format!("Unsupported DIB bit depth: {bpp}")));
    }

    let mut r_mask = 0u32;
    let mut g_mask = 0u32;
    let mut b_mask = 0u32;
    let mut a_mask = 0u32;
    let mut pixel_offset = header_size;

    if compression == 3 || compression == 6 {
        if header_size >= 52 {
            r_mask = read_u32(data, 40);
            g_mask = read_u32(data, 44);
            b_mask = read_u32(data, 48);
            if header_size >= 56 {
                a_mask = read_u32(data, 52);
            }
        } else {
            if data.len() < 40 + 12 {
                return Err(AppError::msg("DIB bit masks are truncated"));
            }
            r_mask = read_u32(data, 40);
            g_mask = read_u32(data, 44);
            b_mask = read_u32(data, 48);
            pixel_offset = 40 + 12;
            if compression == 6 && data.len() >= 56 {
                a_mask = read_u32(data, 52);
                pixel_offset = 40 + 16;
            }
        }
    } else if bpp == 16 {
        r_mask = 0x7C00;
        g_mask = 0x03E0;
        b_mask = 0x001F;
    } else if bpp == 32 {
        r_mask = 0x00FF_0000;
        g_mask = 0x0000_FF00;
        b_mask = 0x0000_00FF;
        a_mask = 0xFF00_0000;
    }

    // clr_used is untrusted clipboard data. Never allocate from it directly:
    // an index DIB has at most 2^bpp entries, and never more than the bytes
    // actually present after the header.
    let palette_count = if bpp <= 8 {
        let maximum = 1usize << bpp;
        let available = data.len().saturating_sub(header_size) / 4;
        if clr_used == 0 {
            maximum.min(available)
        } else {
            (clr_used as usize).min(maximum).min(available)
        }
    } else {
        0
    };

    let mut palette: Vec<[u8; 4]> = Vec::with_capacity(palette_count);
    for index in 0..palette_count {
        let offset = header_size + index * 4;
        if offset + 4 > data.len() {
            break;
        }
        palette.push([data[offset + 2], data[offset + 1], data[offset], 255]);
    }

    if bpp <= 8 {
        pixel_offset = header_size + palette_count * 4;
    }

    let stride = ((width as usize * bpp as usize + 31) / 32) * 4;
    let top_down = height_i < 0;
    let mut out = RgbaImage::new(width, height);
    let mut any_alpha = false;

    for y in 0..height as usize {
        let source_y = if top_down { y } else { height as usize - 1 - y };
        let row_start = pixel_offset + source_y * stride;
        if row_start + stride > data.len() {
            return Err(AppError::msg("DIB pixel data is truncated"));
        }
        for x in 0..width as usize {
            let (r, g, b, a) = match bpp {
                32 => {
                    let offset = row_start + x * 4;
                    (data[offset + 2], data[offset + 1], data[offset], data[offset + 3])
                }
                24 => {
                    let offset = row_start + x * 3;
                    (data[offset + 2], data[offset + 1], data[offset], 255)
                }
                16 => {
                    let offset = row_start + x * 2;
                    let value = u16::from_le_bytes([data[offset], data[offset + 1]]) as u32;
                    (
                        extract_channel(value, r_mask),
                        extract_channel(value, g_mask),
                        extract_channel(value, b_mask),
                        if a_mask != 0 {
                            extract_channel(value, a_mask)
                        } else {
                            255
                        },
                    )
                }
                8 => {
                    let index = data[row_start + x] as usize;
                    let entry = palette.get(index).copied().unwrap_or([0, 0, 0, 255]);
                    (entry[0], entry[1], entry[2], entry[3])
                }
                4 => {
                    let byte = data[row_start + x / 2];
                    let index = if x % 2 == 0 { byte >> 4 } else { byte & 0x0F } as usize;
                    let entry = palette.get(index).copied().unwrap_or([0, 0, 0, 255]);
                    (entry[0], entry[1], entry[2], entry[3])
                }
                1 => {
                    let byte = data[row_start + x / 8];
                    let index = ((byte >> (7 - (x % 8))) & 1) as usize;
                    let entry = palette.get(index).copied().unwrap_or([0, 0, 0, 255]);
                    (entry[0], entry[1], entry[2], entry[3])
                }
                _ => (0, 0, 0, 255),
            };
            if a != 0 {
                any_alpha = true;
            }
            out.put_pixel(x as u32, y as u32, image::Rgba([r, g, b, a]));
        }
    }

    // Many 32-bit DIBs leave the alpha channel at zero and expect it to be opaque.
    if bpp == 32 && !any_alpha {
        for pixel in out.pixels_mut() {
            pixel[3] = 255;
        }
    }

    Ok(out)
}

fn extract_channel(value: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    let raw = (value & mask) >> shift;
    ((raw * 255 + max / 2) / max) as u8
}

fn read_u16(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

fn read_u32(data: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

fn read_i32(data: &[u8], offset: usize) -> i32 {
    i32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], data[offset + 3]])
}

pub fn encode_png(image: &RgbaImage) -> Result<Vec<u8>> {
    let mut buffer = Vec::new();
    PngEncoder::new(&mut buffer).write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        ExtendedColorType::Rgba8,
    )?;
    Ok(buffer)
}

fn fit(image: &RgbaImage, max: u32) -> RgbaImage {
    let (width, height) = image.dimensions();
    if width <= max && height <= max {
        return image.clone();
    }
    let scale = (max as f32 / width as f32).min(max as f32 / height as f32);
    let new_width = ((width as f32 * scale).round() as u32).max(1);
    let new_height = ((height as f32 * scale).round() as u32).max(1);
    image::imageops::resize(image, new_width, new_height, FilterType::CatmullRom)
}

pub fn save_assets(paths: &StoragePaths, uuid: &str, image: &RgbaImage) -> Result<SavedImage> {
    let png = encode_png(image)?;
    let preview = fit(image, 1600);
    let preview_png = encode_png(&preview)?;
    let thumb = fit(image, 360);
    let thumb_png = encode_png(&thumb)?;

    let image_rel = format!("images/{uuid}.png");
    let preview_rel = format!("images/{uuid}-preview.png");
    let thumb_rel = format!("thumbnails/{uuid}.png");

    atomic_write(&paths.asset_path(&image_rel), &png)?;
    atomic_write(&paths.asset_path(&preview_rel), &preview_png)?;
    atomic_write(&paths.asset_path(&thumb_rel), &thumb_png)?;

    Ok(SavedImage {
        png,
        width: image.width(),
        height: image.height(),
        image_rel,
        thumb_rel,
        preview_rel,
    })
}

pub fn to_dibv5(image: &RgbaImage) -> Vec<u8> {
    let width = image.width();
    let height = image.height();
    let mut out = Vec::with_capacity(124 + (width as usize * height as usize * 4));
    let mut header = [0u8; 124];
    header[0..4].copy_from_slice(&124u32.to_le_bytes());
    header[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    header[8..12].copy_from_slice(&(height as i32).to_le_bytes());
    header[12..14].copy_from_slice(&1u16.to_le_bytes());
    header[14..16].copy_from_slice(&32u16.to_le_bytes());
    header[16..20].copy_from_slice(&3u32.to_le_bytes());
    header[20..24].copy_from_slice(&(width * height * 4).to_le_bytes());
    header[24..28].copy_from_slice(&2835i32.to_le_bytes());
    header[28..32].copy_from_slice(&2835i32.to_le_bytes());
    header[40..44].copy_from_slice(&0x00FF_0000u32.to_le_bytes());
    header[44..48].copy_from_slice(&0x0000_FF00u32.to_le_bytes());
    header[48..52].copy_from_slice(&0x0000_00FFu32.to_le_bytes());
    header[52..56].copy_from_slice(&0xFF00_0000u32.to_le_bytes());
    header[56..60].copy_from_slice(&0x7352_4742u32.to_le_bytes()); // LCS_sRGB
    header[108..112].copy_from_slice(&4u32.to_le_bytes()); // LCS_GM_IMAGES
    out.extend_from_slice(&header);
    for y in (0..height).rev() {
        for x in 0..width {
            let pixel = image.get_pixel(x, y);
            out.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }
    out
}

pub fn to_dib(image: &RgbaImage) -> Vec<u8> {
    let width = image.width() as usize;
    let height = image.height() as usize;
    let stride = ((width * 24 + 31) / 32) * 4;
    let mut out = Vec::with_capacity(40 + stride * height);
    let mut header = [0u8; 40];
    header[0..4].copy_from_slice(&40u32.to_le_bytes());
    header[4..8].copy_from_slice(&(width as i32).to_le_bytes());
    header[8..12].copy_from_slice(&(height as i32).to_le_bytes());
    header[12..14].copy_from_slice(&1u16.to_le_bytes());
    header[14..16].copy_from_slice(&24u16.to_le_bytes());
    header[16..20].copy_from_slice(&0u32.to_le_bytes());
    header[20..24].copy_from_slice(&((stride * height) as u32).to_le_bytes());
    header[24..28].copy_from_slice(&2835i32.to_le_bytes());
    header[28..32].copy_from_slice(&2835i32.to_le_bytes());
    out.extend_from_slice(&header);
    for y in (0..height).rev() {
        let start = out.len();
        for x in 0..width {
            let pixel = image.get_pixel(x as u32, y as u32);
            out.extend_from_slice(&[pixel[2], pixel[1], pixel[0]]);
        }
        while out.len() - start < stride {
            out.push(0);
        }
    }
    out
}

const MAX_DECODED_PIXELS: u64 = 120_000_000;
const MAX_ALLOC_BYTES: u64 = 480 * 1024 * 1024;

/// Decode untrusted image bytes with hard allocation limits. A small PNG can
/// otherwise expand to gigabytes of pixel data and abort the process.
fn decode_limited(bytes: &[u8]) -> Result<RgbaImage> {
    let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|err| AppError::msg(format!("image format could not be detected: {err}")))?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(MAX_ALLOC_BYTES);
    limits.max_image_width = Some(20_000);
    limits.max_image_height = Some(20_000);
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|err| AppError::msg(format!("image could not be decoded safely: {err}")))?;
    let rgba = decoded.to_rgba8();
    if rgba.width() as u64 * rgba.height() as u64 > MAX_DECODED_PIXELS {
        return Err(AppError::msg("image is too large to decode safely"));
    }
    Ok(rgba)
}

pub fn decode_file(path: &std::path::Path) -> Result<RgbaImage> {
    let bytes = std::fs::read(path)?;
    decode_limited(&bytes)
}

pub fn decode_bytes(bytes: &[u8]) -> Result<RgbaImage> {
    decode_limited(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_dibv5() {
        let image = RgbaImage::from_pixel(3, 2, image::Rgba([200, 100, 50, 255]));
        let dib = to_dibv5(&image);
        let decoded = parse_dib(&dib).unwrap();
        assert_eq!(decoded.dimensions(), (3, 2));
        assert_eq!(decoded.get_pixel(1, 1), &image::Rgba([200, 100, 50, 255]));
    }

    #[test]
    fn round_trip_dib24() {
        let image = RgbaImage::from_pixel(5, 3, image::Rgba([10, 20, 30, 255]));
        let dib = to_dib(&image);
        let decoded = parse_dib(&dib).unwrap();
        assert_eq!(decoded.dimensions(), (5, 3));
        assert_eq!(decoded.get_pixel(2, 1), &image::Rgba([10, 20, 30, 255]));
    }
}

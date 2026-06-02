use crate::error::AppError;
use crate::models::DocumentMetadata;
use ::image::ImageFormat;
use printpdf::*;
use std::fs::File;
use std::io::{BufWriter, Cursor};
use std::path::Path;

/// Pages processed per batch to limit memory usage.
const BATCH_SIZE: usize = 50;

/// A4 page width in millimeters.
const PAGE_WIDTH_MM: f32 = 210.0;

/// A4 page height in millimeters.
const PAGE_HEIGHT_MM: f32 = 297.0;

/// Generate a PDF from page images in the temp directory.
///
/// Reads WebP files named `{page}.webp` (1-indexed) from `temp_dir`, decodes each
/// to raw RGB, re-encodes as JPEG, and embeds as a full-page image in an A4 PDF
/// document. The result is written to `output_path`.
///
/// Pages are processed in chunks of [`BATCH_SIZE`] to limit memory usage.
///
/// If `metadata` is provided, the PDF will include embedded document info
/// (title, author, source URL).
///
/// # Errors
/// - `AppError::FileSystemError` if `temp_dir` does not exist or a page file is missing.
/// - `AppError::ConversionError` if a page image is malformed or PDF encoding fails.
pub async fn generate(
    temp_dir: &Path,
    page_count: usize,
    output_path: &Path,
    metadata: Option<&DocumentMetadata>,
) -> Result<(), AppError> {
    if !temp_dir.exists() {
        return Err(AppError::FileSystemError(format!(
            "Temp directory does not exist: {}",
            temp_dir.display()
        )));
    }
    if page_count == 0 {
        return Err(AppError::ConversionError(
            "No pages to convert".to_string(),
        ));
    }

    let temp_dir = temp_dir.to_path_buf();
    let output_path = output_path.to_path_buf();
    let metadata = metadata.cloned();

    tokio::task::spawn_blocking(move || generate_sync(&temp_dir, page_count, &output_path, metadata.as_ref()))
        .await
        .map_err(|e| AppError::ConversionError(format!("PDF task join error: {}", e)))?
}

/// Synchronous PDF generation. Runs on a blocking thread because `printpdf` types
/// use `Rc` internally and are not `Send`, so the entire document lifecycle
/// (create, populate, save) must happen on a single thread.
fn generate_sync(
    temp_dir: &Path,
    page_count: usize,
    output_path: &Path,
    metadata: Option<&DocumentMetadata>,
) -> Result<(), AppError> {
    tracing::info!("Starting PDF generation: {} pages", page_count);

    let title = metadata
        .map(|m| m.title.as_str())
        .unwrap_or("AnyflipDL Document");

    let (doc, first_page_idx, first_layer_idx) = PdfDocument::new(
        title,
        Mm(PAGE_WIDTH_MM),
        Mm(PAGE_HEIGHT_MM),
        "Layer 1",
    );

    // Embed metadata if available.
    let doc = if let Some(meta) = metadata {
        let doc = doc.with_creator("AnyflipDL");
        let doc = if let Some(ref author) = meta.author {
            doc.with_author(author)
        } else {
            doc
        };
        doc.with_subject(&meta.url)
    } else {
        doc
    };

    // Pre-compute A4 page size in points for image scaling.
    let page_w_pt: f32 = Mm(PAGE_WIDTH_MM).into_pt().0;
    let page_h_pt: f32 = Mm(PAGE_HEIGHT_MM).into_pt().0;

    // Process pages in batches to bound peak memory.
    let page_range: Vec<usize> = (1..=page_count).collect();
    for (chunk_idx, chunk) in page_range.chunks(BATCH_SIZE).enumerate() {
        tracing::info!(
            "Processing PDF chunk {} (pages {}-{})",
            chunk_idx + 1,
            chunk[0],
            chunk[chunk.len() - 1]
        );

        for &page_num in chunk {
            let page_path = temp_dir.join(format!("{}.webp", page_num));
            let page_data = std::fs::read(&page_path).map_err(|e| {
                AppError::FileSystemError(format!(
                    "Failed to read page {} from {}: {}",
                    page_num,
                    page_path.display(),
                    e
                ))
            })?;

            // Decode the image (WebP or JPEG) into raw RGB pixels, then re-encode
            // as JPEG so we can embed it with the DCT filter. PDF readers expect
            // either raw pixels or DCT-compressed JPEG — embedding compressed WebP
            // bytes as ColorSpace::Rgb produces garbage output.
            let dynamic_img = ::image::load_from_memory(&page_data).map_err(|e| {
                AppError::ConversionError(format!(
                    "Failed to decode page {} image: {}",
                    page_num, e
                ))
            })?;
            let rgb_img = dynamic_img.to_rgb8();
            let (width, height) = rgb_img.dimensions();

            let mut jpeg_buf: Vec<u8> = Vec::new();
            rgb_img
                .write_to(&mut Cursor::new(&mut jpeg_buf), ImageFormat::Jpeg)
                .map_err(|e| {
                    AppError::ConversionError(format!(
                        "Failed to encode page {} as JPEG: {}",
                        page_num, e
                    ))
                })?;

            let image_xobject = ImageXObject {
                width: Px(width as usize),
                height: Px(height as usize),
                color_space: ColorSpace::Rgb,
                bits_per_component: ColorBits::Bit8,
                interpolate: true,
                image_data: jpeg_buf,
                image_filter: Some(ImageFilter::DCT),
                smask: None,
                clipping_bbox: None,
            };
            let image = Image::from(image_xobject);

            // Scale to fit A4 while preserving aspect ratio.
            // At 72 DPI, 1 pixel == 1 point.
            let dpi = 72.0_f32;
            let img_w_pt = width as f32;
            let img_h_pt = height as f32;
            let scale = (page_w_pt / img_w_pt).min(page_h_pt / img_h_pt);

            // The first page was created by PdfDocument::new; add subsequent pages.
            let (page_idx, layer_idx) = if page_num == 1 {
                (first_page_idx, first_layer_idx)
            } else {
                doc.add_page(Mm(PAGE_WIDTH_MM), Mm(PAGE_HEIGHT_MM), "Layer 1")
            };

            let layer = doc.get_page(page_idx).get_layer(layer_idx);
            image.add_to_layer(
                layer,
                ImageTransform {
                    dpi: Some(dpi),
                    scale_x: Some(scale),
                    scale_y: Some(scale),
                    ..Default::default()
                },
            );
        }
    }

    let file = File::create(output_path).map_err(|e| {
        AppError::FileSystemError(format!(
            "Failed to create output PDF at {}: {}",
            output_path.display(),
            e
        ))
    })?;
    doc.save(&mut BufWriter::new(file))?;

    tracing::info!("PDF saved to {}", output_path.display());
    Ok(())
}

/// Parse image dimensions from raw file bytes. Supports both JPEG and WebP formats.
///
/// For JPEG: scans for the SOF marker (0xC0-0xCF) which contains height and width.
/// For WebP: reads dimensions from the VP8 chunk header at bytes 16-23.
///
/// Note: `generate_sync` now uses the `image` crate for decoding, so this function
/// is only exercised by unit tests but kept as a lightweight fallback/verification path.
#[allow(dead_code)]
fn parse_image_dimensions(data: &[u8]) -> Result<(u32, u32), AppError> {
    if data.len() < 12 {
        return Err(AppError::ConversionError(
            "Image data too short to determine format".to_string(),
        ));
    }

    // Check for WebP format: RIFF....WEBP
    if data.len() >= 24 && &data[0..4] == b"RIFF" && &data[8..12] == b"WEBP" {
        // VP8 lossy: dimensions at bytes 16-23 (little-endian u16)
        // VP8L lossless: dimensions at bytes 21-24
        // VP8X extended: dimensions at bytes 20-26
        // For simplicity, try VP8 lossy first (most common)
        if &data[12..16] == b"VP8 " && data.len() >= 30 {
            let width = u16::from_le_bytes([data[26], data[27]]) as u32;
            let height = u16::from_le_bytes([data[28], data[29]]) as u32;
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
        // VP8L lossless
        if &data[12..16] == b"VP8L" && data.len() >= 25 {
            let bits = u32::from_le_bytes([data[21], data[22], data[23], data[24]]);
            let width = (bits & 0x3FFF) + 1;
            let height = ((bits >> 14) & 0x3FFF) + 1;
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
        // VP8X extended
        if &data[12..16] == b"VP8X" && data.len() >= 30 {
            let width = (u32::from_le_bytes([data[24], data[25], data[26], 0]) & 0xFFFFFF) + 1;
            let height = (u32::from_le_bytes([data[27], data[28], data[29], 0]) & 0xFFFFFF) + 1;
            if width > 0 && height > 0 {
                return Ok((width, height));
            }
        }
        return Err(AppError::ConversionError(
            "Unsupported WebP variant".to_string(),
        ));
    }

    // Fall back to JPEG parsing
    if data.len() < 4 || data[0] != 0xFF || data[1] != 0xD8 {
        return Err(AppError::ConversionError(
            "Invalid image data: not JPEG or WebP".to_string(),
        ));
    }

    let mut pos = 2;
    while pos + 2 <= data.len() {
        if data[pos] != 0xFF {
            return Err(AppError::ConversionError(format!(
                "Invalid JPEG: expected marker byte 0xFF at position {}, got 0x{:02X}",
                pos, data[pos]
            )));
        }

        let marker = data[pos + 1];
        pos += 2;

        // Standalone markers (no length field).
        match marker {
            0x00 | 0x01 | 0xD0..=0xD7 => continue,
            0xD8 => continue, // Embedded SOI (rare but valid)
            0xD9 => {
                return Err(AppError::ConversionError(
                    "JPEG EOI reached before finding SOF marker".to_string(),
                ));
            }
            _ => {}
        }

        // Markers with a 2-byte length field.
        if pos + 2 > data.len() {
            return Err(AppError::ConversionError(
                "JPEG data truncated: expected marker length".to_string(),
            ));
        }

        let length = u16::from_be_bytes([data[pos], data[pos + 1]]) as usize;

        // SOF markers: 0xC0-0xC3, 0xC5-0xC7, 0xC9-0xCB, 0xCD-0xCF
        // (excludes 0xC4=DHT, 0xC8=JPG, 0xCC=DAC)
        if matches!(
            marker,
            0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF
        ) {
            if length < 7 || pos + 2 + 7 > data.len() {
                return Err(AppError::ConversionError(
                    "JPEG SOF segment truncated".to_string(),
                ));
            }
            // Byte layout after length: precision(1), height(2), width(2)
            let height = u16::from_be_bytes([data[pos + 3], data[pos + 4]]);
            let width = u16::from_be_bytes([data[pos + 5], data[pos + 6]]);
            return Ok((width as u32, height as u32));
        }

        // Skip to next marker.
        if length < 2 || pos + length > data.len() {
            return Err(AppError::ConversionError(
                "JPEG marker length extends beyond data".to_string(),
            ));
        }
        pos += length;
    }

    Err(AppError::ConversionError(
        "Could not find JPEG SOF marker — image may be malformed".to_string(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Minimal valid JPEG header: SOI + SOF0 for a 100x50 image.
    fn make_test_jpeg(width: u16, height: u16) -> Vec<u8> {
        let mut data = Vec::new();
        // SOI
        data.extend_from_slice(&[0xFF, 0xD8]);
        // APP0 marker (filler segment)
        data.extend_from_slice(&[0xFF, 0xE0]);
        data.extend_from_slice(&16u16.to_be_bytes()); // length = 16
        data.extend_from_slice(&[0; 14]); // dummy JFIF data
        // SOF0 marker
        data.extend_from_slice(&[0xFF, 0xC0]);
        data.extend_from_slice(&11u16.to_be_bytes()); // length = 11
        data.push(8); // precision
        data.extend_from_slice(&height.to_be_bytes());
        data.extend_from_slice(&width.to_be_bytes());
        data.push(1); // number of components
        data.push(1); // component id
        data.push(0x11); // sampling
        data.push(0); // quant table
        // EOI
        data.extend_from_slice(&[0xFF, 0xD9]);
        data
    }

    #[test]
    fn test_parse_image_dimensions_valid() {
        let data = make_test_jpeg(640, 480);
        let (w, h) = parse_image_dimensions(&data).unwrap();
        assert_eq!(w, 640);
        assert_eq!(h, 480);
    }

    #[test]
    fn test_parse_image_dimensions_small_image() {
        let data = make_test_jpeg(1, 1);
        let (w, h) = parse_image_dimensions(&data).unwrap();
        assert_eq!(w, 1);
        assert_eq!(h, 1);
    }

    #[test]
    fn test_parse_image_dimensions_large_image() {
        let data = make_test_jpeg(8000, 6000);
        let (w, h) = parse_image_dimensions(&data).unwrap();
        assert_eq!(w, 8000);
        assert_eq!(h, 6000);
    }

    #[test]
    fn test_parse_image_dimensions_invalid_soi() {
        let data = vec![0x00, 0x00, 0x00, 0x00];
        assert!(parse_image_dimensions(&data).is_err());
    }

    #[test]
    fn test_parse_image_dimensions_too_short() {
        let data = vec![0xFF, 0xD8];
        assert!(parse_image_dimensions(&data).is_err());
    }

    #[test]
    fn test_parse_image_dimensions_no_sof() {
        // SOI + EOI only — no SOF marker.
        let data = vec![0xFF, 0xD8, 0xFF, 0xD9];
        assert!(parse_image_dimensions(&data).is_err());
    }
}

use crate::error::AppError;
use crate::models::DocumentMetadata;
use epub_builder::{EpubBuilder, EpubContent, EpubVersion, ReferenceType, ZipLibrary};
use std::io::Write;
use std::path::Path;

/// Minimal CSS so each page image fills the viewport.
const EPUB_STYLESHEET: &str = r#"body {
    margin: 0;
    padding: 0;
    text-align: center;
}
img {
    max-width: 100%;
    height: auto;
}"#;

/// Generate an EPUB from page images in the temp directory.
///
/// Reads WebP files named `{page}.webp` (1-indexed) from `temp_dir`, embeds each
/// as a full-page image in an EPUB document, and writes the result to `output_path`.
///
/// Images are added incrementally — each page image is read from disk, passed to
/// the EPUB builder, and then dropped before the next page is loaded. This keeps
/// memory usage constant regardless of document size.
///
/// If `metadata` is provided, the EPUB will include embedded document info
/// (title, author, source URL).
///
/// # Errors
/// - `AppError::FileSystemError` if `temp_dir` does not exist or a page file is missing.
/// - `AppError::ConversionError` if EPUB encoding fails.
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
        .map_err(|e| AppError::ConversionError(format!("EPUB task join error: {}", e)))?
}

/// Synchronous EPUB generation. Runs on a blocking thread because the EPUB build
/// involves sequential I/O (reading images, writing to the zip archive).
fn generate_sync(
    temp_dir: &Path,
    page_count: usize,
    output_path: &Path,
    metadata: Option<&DocumentMetadata>,
) -> Result<(), AppError> {
    tracing::info!("Starting EPUB generation: {} pages", page_count);

    let zip = ZipLibrary::new().map_err(|e| {
        AppError::ConversionError(format!("Failed to create EPUB zip library: {}", e))
    })?;
    let mut builder = EpubBuilder::new(zip).map_err(|e| {
        AppError::ConversionError(format!("Failed to create EPUB builder: {}", e))
    })?;

    // Set metadata.
    let title = metadata
        .map(|m| m.title.as_str())
        .unwrap_or("AnyflipDL Document");
    builder
        .metadata("title", title)
        .map_err(epub_err)?;
    builder
        .metadata("lang", "en")
        .map_err(epub_err)?;

    // Embed author and source URL if available.
    if let Some(meta) = metadata {
        if let Some(ref author) = meta.author {
            builder
                .metadata("creator", author.as_str())
                .map_err(epub_err)?;
        }
        builder
            .metadata("source", &meta.url)
            .map_err(epub_err)?;
        builder
            .metadata("description", format!("Converted from Anyflip: {}", &meta.url))
            .map_err(epub_err)?;
    }

    builder
        .metadata("publisher", "AnyflipDL")
        .map_err(epub_err)?;
    builder.epub_version(EpubVersion::V30);

    // Embed a stylesheet so images fill the page.
    builder
        .stylesheet(EPUB_STYLESHEET.as_bytes())
        .map_err(epub_err)?;

    // Add cover image (first page) and remaining pages incrementally.
    for page_num in 1..=page_count {
        let page_path = temp_dir.join(format!("{}.webp", page_num));
        let page_data = std::fs::read(&page_path).map_err(|e| {
            AppError::FileSystemError(format!(
                "Failed to read page {} from {}: {}",
                page_num,
                page_path.display(),
                e
            ))
        })?;

        // Add the image as a resource inside the EPUB.
        let image_href = format!("images/{}.webp", page_num);
        builder
            .add_resource(&image_href[..], page_data.as_slice(), "image/webp")
            .map_err(epub_err)?;

        // Build the XHTML content that references the image.
        let xhtml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE html>
<html xmlns="http://www.w3.org/1999/xhtml">
<head>
  <title>Page {page}</title>
  <link rel="stylesheet" type="text/css" href="stylesheet.css"/>
</head>
<body>
  <img src="{src}" alt="Page {page}" type="image/webp"/>
</body>
</html>"#,
            page = page_num,
            src = image_href,
        );

        let content_href = format!("page_{}.xhtml", page_num);
        let reftype = if page_num == 1 {
            Some(ReferenceType::Cover)
        } else {
            Some(ReferenceType::Text)
        };

        let mut epub_content =
            EpubContent::new(&content_href[..], xhtml.as_bytes()).title(format!("Page {}", page_num));
        if let Some(rt) = reftype {
            epub_content = epub_content.reftype(rt);
        }

        builder
            .add_content(epub_content)
            .map_err(epub_err)?;

        if page_num % 50 == 0 || page_num == page_count {
            tracing::info!("EPUB: added page {}/{}", page_num, page_count);
        }
    }

    // Write the EPUB file.
    let mut output_file = std::fs::File::create(output_path).map_err(|e| {
        AppError::FileSystemError(format!(
            "Failed to create output EPUB at {}: {}",
            output_path.display(),
            e
        ))
    })?;

    builder.generate(&mut output_file).map_err(epub_err)?;

    output_file.flush().map_err(|e| {
        AppError::FileSystemError(format!("Failed to flush EPUB output: {}", e))
    })?;

    tracing::info!("EPUB saved to {}", output_path.display());
    Ok(())
}

/// Convert an `eyre::Report` (used by `epub-builder`) into an `AppError`.
fn epub_err(e: impl std::fmt::Display) -> AppError {
    AppError::ConversionError(e.to_string())
}

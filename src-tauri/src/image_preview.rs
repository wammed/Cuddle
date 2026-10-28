// Cuddle - Phase 10: Rich Image Preview Module
// Supports Native formats: PNG, JPEG, SVG, WebP, GIF, AVIF, BMP, ICO, TIFF
// Extended formats (via Embedded JPEG / External Tool Delegation): RAW (CR2/NEF/ARW/DNG/RW2/ORF/PEF/RAF), HEIC/HEIF, PSD, HDR

use crate::models::{decode_path_from_base64, FileItem};
use base64::{engine::general_purpose::STANDARD as BASE64_STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use thiserror::Error;
use ts_rs::TS;

/// Maximum image file size loaded into preview memory (50 MiB guardrail)
pub const DEFAULT_MAX_IMAGE_BYTES: usize = 50 * 1024 * 1024;

#[derive(Error, Debug)]
pub enum ImagePreviewError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Model error: {0}")]
    Model(#[from] crate::models::ModelError),
    #[error("File size exceeds maximum preview limit ({0} bytes > {1} bytes)")]
    FileSizeExceeded(u64, usize),
    #[error("Unsupported image extension: {0}")]
    UnsupportedFormat(String),
    #[error("File is a directory")]
    IsDirectory,
    #[error("External tool required for {format} preview: please install {tool_hint}")]
    ToolRequired { format: String, tool_hint: String },
    #[error("External image conversion failed: {0}")]
    ConversionFailed(String),
}

/// EXIF metadata extracted from supported image formats (JPEG, TIFF, WebP, RAW)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ImageExifDto {
    pub make: Option<String>,
    pub model: Option<String>,
    pub date_time: Option<String>,
    pub exposure_time: Option<String>,
    pub f_number: Option<String>,
    pub iso: Option<String>,
    pub focal_length: Option<String>,
    pub lens_model: Option<String>,
}

/// Comprehensive image preview data transfer object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[ts(export, export_to = "../../src/types/")]
pub struct ImagePreviewDto {
    pub item_id: String,
    pub raw_base64: String,
    pub display_name: String,
    pub mime_type: String,
    pub original_format: String,
    pub data_url: String,
    pub file_size: u64,
    pub is_svg: bool,
    pub svg_content: Option<String>,
    pub is_converted: bool,
    pub exif: Option<ImageExifDto>,
}

/// Determine whether the extension is a camera RAW file.
pub fn is_raw_image_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cr2" | "cr3" | "nef" | "arw" | "dng" | "rw2" | "orf" | "pef" | "raf"
    )
}

/// Determine whether the format requires external tool delegation (patent-free safety).
pub fn is_external_delegation_extension(ext: &str) -> bool {
    let lower = ext.to_ascii_lowercase();
    matches!(lower.as_str(), "heic" | "heif" | "psd" | "hdr")
}

/// Determine MIME type for supported image extension.
/// Supports: PNG, APNG, JPEG, JPG, JFIF, SVG, SVGZ, WEBP, GIF, AVIF, BMP, DIB, ICO, TIFF, TIF,
/// plus extended formats: HEIC, HEIF, RAW (CR2, NEF, ARW, DNG, etc.), PSD, HDR.
pub fn get_image_mime_type(ext: &str) -> Option<&'static str> {
    let lower = ext.to_ascii_lowercase();
    match lower.as_str() {
        "png" | "apng" => Some("image/png"),
        "jpg" | "jpeg" | "jfif" | "pjpeg" | "pjp" => Some("image/jpeg"),
        "svg" | "svgz" => Some("image/svg+xml"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        "avif" => Some("image/avif"),
        "bmp" | "dib" => Some("image/bmp"),
        "ico" => Some("image/x-icon"),
        "tiff" | "tif" => Some("image/tiff"),
        "heic" | "heif" => Some("image/heic"),
        "psd" => Some("image/vnd.adobe.photoshop"),
        "hdr" => Some("image/vnd.radiance"),
        "cr2" | "cr3" | "nef" | "arw" | "dng" | "rw2" | "orf" | "pef" | "raf" => {
            Some("image/x-raw")
        }
        _ => None,
    }
}

/// Check whether the extension belongs to any supported previewable image (native or extended).
pub fn is_supported_image_extension(ext: &str) -> bool {
    get_image_mime_type(ext).is_some()
}

/// Extract EXIF metadata if available using kamadak-exif
pub fn extract_image_exif(path: &Path) -> Option<ImageExifDto> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);

    let exifreader = exif::Reader::new();
    let exif = exifreader.read_from_container(&mut reader).ok()?;

    let get_tag_str = |tag: exif::Tag| -> Option<String> {
        exif.get_field(tag, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string().trim().to_string())
    };

    let make = get_tag_str(exif::Tag::Make);
    let model = get_tag_str(exif::Tag::Model);
    let date_time = get_tag_str(exif::Tag::DateTimeOriginal)
        .or_else(|| get_tag_str(exif::Tag::DateTimeDigitized))
        .or_else(|| get_tag_str(exif::Tag::DateTime));
    let exposure_time = get_tag_str(exif::Tag::ExposureTime);
    let f_number = get_tag_str(exif::Tag::FNumber);
    let iso = get_tag_str(exif::Tag::PhotographicSensitivity);
    let focal_length = get_tag_str(exif::Tag::FocalLength);
    let lens_model = get_tag_str(exif::Tag::LensModel);

    if make.is_none()
        && model.is_none()
        && date_time.is_none()
        && exposure_time.is_none()
        && f_number.is_none()
        && iso.is_none()
        && focal_length.is_none()
        && lens_model.is_none()
    {
        None
    } else {
        Some(ImageExifDto {
            make,
            model,
            date_time,
            exposure_time,
            f_number,
            iso,
            focal_length,
            lens_model,
        })
    }
}

/// Extract embedded JPEG preview from camera RAW files without developing the raw sensor data.
/// High performance & patent-free approach.
pub fn extract_raw_embedded_jpeg(path: &Path) -> Option<Vec<u8>> {
    // 1. Try exiftool if available on the system
    if let Some(exiftool) = crate::pipeline::ToolDetector::find_binary_in_path("exiftool") {
        if let Ok(output) = std::process::Command::new(&exiftool)
            .args(["-b", "-PreviewImage"])
            .arg(path)
            .output()
        {
            if !output.stdout.is_empty() && output.stdout.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return Some(output.stdout);
            }
        }

        if let Ok(output) = std::process::Command::new(&exiftool)
            .args(["-b", "-JpgFromRaw"])
            .arg(path)
            .output()
        {
            if !output.stdout.is_empty() && output.stdout.starts_with(&[0xFF, 0xD8, 0xFF]) {
                return Some(output.stdout);
            }
        }
    }

    // 2. Built-in TIFF / Exif stream scan for embedded JPEG SOI/EOI
    if let Ok(mut file) = File::open(path) {
        let mut buffer = Vec::new();
        // Limit initial search to max 16 MiB for responsive preview
        let mut handle = (&mut file).take(16 * 1024 * 1024);
        if handle.read_to_end(&mut buffer).is_ok() && buffer.len() > 1024 {
            let mut best_start = None;
            let mut best_len = 0;

            let mut i = 0;
            while i + 3 < buffer.len() {
                if buffer[i] == 0xFF && buffer[i + 1] == 0xD8 && buffer[i + 2] == 0xFF {
                    let start = i;
                    let mut j = i + 2;
                    let mut found_eoi = None;
                    while j + 1 < buffer.len() {
                        if buffer[j] == 0xFF && buffer[j + 1] == 0xD9 {
                            found_eoi = Some(j + 2);
                        }
                        j += 1;
                    }

                    if let Some(eoi) = found_eoi {
                        let len = eoi - start;
                        if len > best_len && len > 5 * 1024 {
                            best_len = len;
                            best_start = Some((start, eoi));
                        }
                    }
                }
                i += 1;
            }

            if let Some((start, end)) = best_start {
                return Some(buffer[start..end].to_vec());
            }
        }
    }

    None
}

/// Convert formats (HEIC, PSD, HDR, or fallback) via system external tool (magick/convert/ffmpeg).
/// Ensures patent safety by delegating to host system binaries.
pub fn convert_via_external_tool(
    path: &Path,
    format_label: &str,
) -> Result<(Vec<u8>, &'static str), ImagePreviewError> {
    let tool_bin = crate::pipeline::ToolDetector::find_binary_in_path("magick")
        .or_else(|| crate::pipeline::ToolDetector::find_binary_in_path("convert"))
        .or_else(|| crate::pipeline::ToolDetector::find_binary_in_path("ffmpeg"));

    let tool_path = tool_bin.ok_or_else(|| ImagePreviewError::ToolRequired {
        format: format_label.to_string(),
        tool_hint: "ImageMagick (magick) or FFmpeg".to_string(),
    })?;

    let tool_name = tool_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("tool");

    let temp_dir = std::env::temp_dir();
    let temp_out = temp_dir.join(format!("cuddle_preview_{}.png", uuid::Uuid::new_v4()));

    let cmd_result = if tool_name == "ffmpeg" {
        std::process::Command::new(&tool_path)
            .args(["-y", "-i"])
            .arg(path)
            .args(["-frames:v", "1", "-update", "1"])
            .arg(&temp_out)
            .output()
    } else {
        // magick or convert: specify input[0] for first layer/page
        let mut input_spec = path.as_os_str().to_os_string();
        input_spec.push("[0]");
        std::process::Command::new(&tool_path)
            .arg(input_spec)
            .arg(&temp_out)
            .output()
    };

    let output = cmd_result.map_err(|e| ImagePreviewError::ConversionFailed(e.to_string()))?;

    if !output.status.success() || !temp_out.exists() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let _ = std::fs::remove_file(&temp_out);
        return Err(ImagePreviewError::ConversionFailed(format!(
            "Tool execution exited with error: {}",
            stderr.trim()
        )));
    }

    let mut out_file = File::open(&temp_out)?;
    let mut bytes = Vec::new();
    out_file.read_to_end(&mut bytes)?;
    let _ = std::fs::remove_file(&temp_out);

    Ok((bytes, "image/png"))
}

/// Safely load image preview data for a given filesystem path.
pub fn load_image_preview_from_path(
    path: &Path,
    max_bytes: Option<usize>,
) -> Result<ImagePreviewDto, ImagePreviewError> {
    let item = FileItem::from_path(path)?;
    if item.is_dir {
        return Err(ImagePreviewError::IsDirectory);
    }

    let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
    let original_format = ext.to_ascii_uppercase();
    let raw_mime = get_image_mime_type(ext)
        .ok_or_else(|| ImagePreviewError::UnsupportedFormat(ext.to_string()))?;

    let limit = max_bytes.unwrap_or(DEFAULT_MAX_IMAGE_BYTES);
    let meta = std::fs::symlink_metadata(path)?;
    let file_size = meta.len();

    if file_size > limit as u64 {
        return Err(ImagePreviewError::FileSizeExceeded(file_size, limit));
    }

    let exif = extract_image_exif(path);

    // Strategy 1: Camera RAW handling (Embedded JPEG extraction with external fallback)
    if is_raw_image_extension(ext) {
        if let Some(jpeg_bytes) = extract_raw_embedded_jpeg(path) {
            let encoded = BASE64_STANDARD.encode(&jpeg_bytes);
            let data_url = format!("data:image/jpeg;base64,{}", encoded);
            return Ok(ImagePreviewDto {
                item_id: item.item_id,
                raw_base64: item.raw_base64,
                display_name: item.display_name,
                mime_type: "image/jpeg".to_string(),
                original_format,
                data_url,
                file_size,
                is_svg: false,
                svg_content: None,
                is_converted: true,
                exif,
            });
        }

        // If no embedded JPEG could be extracted, delegate to external tool
        let (converted_bytes, mime) = convert_via_external_tool(path, "RAW")?;
        let encoded = BASE64_STANDARD.encode(&converted_bytes);
        let data_url = format!("data:{};base64,{}", mime, encoded);
        return Ok(ImagePreviewDto {
            item_id: item.item_id,
            raw_base64: item.raw_base64,
            display_name: item.display_name,
            mime_type: mime.to_string(),
            original_format,
            data_url,
            file_size,
            is_svg: false,
            svg_content: None,
            is_converted: true,
            exif,
        });
    }

    // Strategy 2: Formats requiring external delegation (HEIC/HEIF/PSD/HDR)
    if is_external_delegation_extension(ext) {
        let (converted_bytes, mime) = convert_via_external_tool(path, &original_format)?;
        let encoded = BASE64_STANDARD.encode(&converted_bytes);
        let data_url = format!("data:{};base64,{}", mime, encoded);
        return Ok(ImagePreviewDto {
            item_id: item.item_id,
            raw_base64: item.raw_base64,
            display_name: item.display_name,
            mime_type: mime.to_string(),
            original_format,
            data_url,
            file_size,
            is_svg: false,
            svg_content: None,
            is_converted: true,
            exif,
        });
    }

    // Strategy 3: Standard Native Web formats (PNG, JPEG, SVG, WebP, GIF, AVIF, BMP, ICO, TIFF)
    let mut file = File::open(path)?;
    let mut buffer = Vec::with_capacity(file_size as usize);
    file.read_to_end(&mut buffer)?;

    let is_svg = raw_mime == "image/svg+xml";
    let svg_content = if is_svg {
        String::from_utf8(buffer.clone()).ok()
    } else {
        None
    };

    let encoded = BASE64_STANDARD.encode(&buffer);
    let data_url = format!("data:{};base64,{}", raw_mime, encoded);

    Ok(ImagePreviewDto {
        item_id: item.item_id,
        raw_base64: item.raw_base64,
        display_name: item.display_name,
        mime_type: raw_mime.to_string(),
        original_format,
        data_url,
        file_size,
        is_svg,
        svg_content,
        is_converted: false,
        exif,
    })
}

/// Entry point taking a base64-encoded path
pub fn load_image_preview_from_base64(
    path_base64: &str,
    max_bytes: Option<usize>,
) -> Result<ImagePreviewDto, ImagePreviewError> {
    let path = decode_path_from_base64(path_base64)?;
    load_image_preview_from_path(&path, max_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cuddle_img_test_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn test_image_mime_types() {
        assert_eq!(get_image_mime_type("png"), Some("image/png"));
        assert_eq!(get_image_mime_type("PNG"), Some("image/png"));
        assert_eq!(get_image_mime_type("jpg"), Some("image/jpeg"));
        assert_eq!(get_image_mime_type("jpeg"), Some("image/jpeg"));
        assert_eq!(get_image_mime_type("svg"), Some("image/svg+xml"));
        assert_eq!(get_image_mime_type("webp"), Some("image/webp"));
        assert_eq!(get_image_mime_type("gif"), Some("image/gif"));
        assert_eq!(get_image_mime_type("avif"), Some("image/avif"));
        assert_eq!(get_image_mime_type("bmp"), Some("image/bmp"));
        assert_eq!(get_image_mime_type("ico"), Some("image/x-icon"));
        assert_eq!(get_image_mime_type("heic"), Some("image/heic"));
        assert_eq!(get_image_mime_type("heif"), Some("image/heic"));
        assert_eq!(
            get_image_mime_type("psd"),
            Some("image/vnd.adobe.photoshop")
        );
        assert_eq!(get_image_mime_type("hdr"), Some("image/vnd.radiance"));
        assert_eq!(get_image_mime_type("cr2"), Some("image/x-raw"));
        assert_eq!(get_image_mime_type("nef"), Some("image/x-raw"));
        assert_eq!(get_image_mime_type("dng"), Some("image/x-raw"));
        assert_eq!(get_image_mime_type("txt"), None);
    }

    #[test]
    fn test_raw_and_external_extension_helpers() {
        assert!(is_raw_image_extension("cr2"));
        assert!(is_raw_image_extension("NEF"));
        assert!(is_raw_image_extension("arw"));
        assert!(is_raw_image_extension("dng"));
        assert!(!is_raw_image_extension("png"));

        assert!(is_external_delegation_extension("heic"));
        assert!(is_external_delegation_extension("HEIF"));
        assert!(is_external_delegation_extension("psd"));
        assert!(is_external_delegation_extension("hdr"));
        assert!(!is_external_delegation_extension("jpg"));
    }

    #[test]
    fn test_load_png_preview() {
        let dir = create_test_dir();
        let file_path = dir.join("sample.png");
        let png_bytes = [
            0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x00, 0x00, 0x0d, 0x49, 0x48,
            0x44, 0x52, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x08, 0x06, 0x00, 0x00,
            0x00, 0x1f, 0x15, 0xc4, 0x89, 0x00, 0x00, 0x00, 0x0a, 0x49, 0x44, 0x41, 0x54, 0x78,
            0x9c, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, 0x0d, 0x0a, 0x2d, 0xb4, 0x00,
            0x00, 0x00, 0x00, 0x49, 0x45, 0x4e, 0x44, 0xae, 0x42, 0x60, 0x82,
        ];
        std::fs::write(&file_path, &png_bytes).unwrap();

        let preview = load_image_preview_from_path(&file_path, None).unwrap();
        assert_eq!(preview.display_name, "sample.png");
        assert_eq!(preview.mime_type, "image/png");
        assert_eq!(preview.original_format, "PNG");
        assert!(!preview.is_converted);
        assert!(preview.data_url.starts_with("data:image/png;base64,"));
        assert_eq!(preview.file_size, png_bytes.len() as u64);
        assert!(!preview.is_svg);
        assert!(preview.svg_content.is_none());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_extract_raw_embedded_jpeg_stream() {
        let dir = create_test_dir();
        let file_path = dir.join("test_camera.cr2");

        // Construct mock RAW file with header and an embedded JPEG stream (> 5KB)
        let mut raw_mock = vec![0x49, 0x49, 0x2A, 0x00]; // TIFF LE header
        raw_mock.resize(1024, 0xAA); // Some RAW metadata padding
        let jpeg_soi = [0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46];
        raw_mock.extend_from_slice(&jpeg_soi);
        raw_mock.resize(raw_mock.len() + 6000, 0x55); // Embedded JPEG payload
        let jpeg_eoi = [0xFF, 0xD9];
        raw_mock.extend_from_slice(&jpeg_eoi);
        raw_mock.resize(raw_mock.len() + 2048, 0xBB); // Trailing RAW sensor payload

        std::fs::write(&file_path, &raw_mock).unwrap();

        let preview = load_image_preview_from_path(&file_path, None).unwrap();
        assert_eq!(preview.display_name, "test_camera.cr2");
        assert_eq!(preview.original_format, "CR2");
        assert_eq!(preview.mime_type, "image/jpeg");
        assert!(preview.is_converted);
        assert!(preview.data_url.starts_with("data:image/jpeg;base64,"));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_heic_delegation_requires_tool_when_missing() {
        let dir = create_test_dir();
        let file_path = dir.join("iphone.heic");
        std::fs::write(&file_path, b"mock heic bytes").unwrap();

        let res = load_image_preview_from_path(&file_path, None);
        // If magick or ffmpeg is present on the testing host, it will either succeed or fail conversion.
        // If neither is present, it must return ToolRequired.
        match res {
            Ok(preview) => {
                assert_eq!(preview.display_name, "iphone.heic");
                assert_eq!(preview.original_format, "HEIC");
            }
            Err(ImagePreviewError::ToolRequired { format, .. }) => {
                assert_eq!(format, "HEIC");
            }
            Err(ImagePreviewError::ConversionFailed(_)) => {
                // Tool was present on system but mock bytes were invalid HEIC
            }
            Err(e) => panic!("Unexpected error: {:?}", e),
        }

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn test_export_typescript_bindings_image_preview() {
        let _ = std::fs::create_dir_all("../src/types");
        let cfg = ts_rs::Config::default();
        ImagePreviewDto::export(&cfg).expect("Failed to export ImagePreviewDto");
        ImageExifDto::export(&cfg).expect("Failed to export ImageExifDto");
    }
}

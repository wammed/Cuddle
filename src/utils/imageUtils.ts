// Utility functions for image format detection, display metadata, and aspect ratio calculation

const NATIVE_IMAGE_EXTENSIONS = new Set([
  "png",
  "apng",
  "jpg",
  "jpeg",
  "jfif",
  "pjpeg",
  "pjp",
  "svg",
  "svgz",
  "webp",
  "gif",
  "avif",
  "bmp",
  "dib",
  "ico",
  "tiff",
  "tif",
]);

const RAW_IMAGE_EXTENSIONS = new Set([
  "cr2",
  "cr3",
  "nef",
  "arw",
  "dng",
  "rw2",
  "orf",
  "pef",
  "raf",
]);

const EXTERNAL_IMAGE_EXTENSIONS = new Set([
  "heic",
  "heif",
  "psd",
  "hdr",
]);

const ALL_SUPPORTED_EXTENSIONS = new Set([
  ...NATIVE_IMAGE_EXTENSIONS,
  ...RAW_IMAGE_EXTENSIONS,
  ...EXTERNAL_IMAGE_EXTENSIONS,
]);

/**
 * Checks whether a given filename has a supported image extension (native or extended).
 */
export function isImageFilename(filename: string): boolean {
  if (!filename) return false;
  const parts = filename.split(".");
  if (parts.length < 2) return false;
  const ext = parts[parts.length - 1].toLowerCase();
  return ALL_SUPPORTED_EXTENSIONS.has(ext);
}

/**
 * Checks whether a given filename is a camera RAW format.
 */
export function isRawFilename(filename: string): boolean {
  if (!filename) return false;
  const parts = filename.split(".");
  if (parts.length < 2) return false;
  const ext = parts[parts.length - 1].toLowerCase();
  return RAW_IMAGE_EXTENSIONS.has(ext);
}

/**
 * Checks whether a given format requires external tool delegation (HEIC/HEIF/PSD/HDR).
 */
export function isExternalDelegationFilename(filename: string): boolean {
  if (!filename) return false;
  const parts = filename.split(".");
  if (parts.length < 2) return false;
  const ext = parts[parts.length - 1].toLowerCase();
  return EXTERNAL_IMAGE_EXTENSIONS.has(ext);
}

/**
 * Returns a human-friendly uppercase format label from mime type or filename.
 */
export function getImageFormatLabel(mimeType?: string, filename?: string): string {
  if (mimeType) {
    if (mimeType === "image/png") return "PNG";
    if (mimeType === "image/jpeg") return "JPEG";
    if (mimeType === "image/svg+xml") return "SVG";
    if (mimeType === "image/webp") return "WEBP";
    if (mimeType === "image/gif") return "GIF";
    if (mimeType === "image/avif") return "AVIF";
    if (mimeType === "image/bmp") return "BMP";
    if (mimeType === "image/x-icon") return "ICO";
    if (mimeType === "image/tiff") return "TIFF";
    if (mimeType === "image/heic") return "HEIC";
    if (mimeType === "image/vnd.adobe.photoshop") return "PSD";
    if (mimeType === "image/vnd.radiance") return "HDR";
    if (mimeType === "image/x-raw") return "RAW";
  }

  if (filename && filename.includes(".")) {
    const ext = filename.split(".").pop()?.toUpperCase();
    if (ext) return ext;
  }

  return "IMAGE";
}

/**
 * Formats bytes into a human-readable size string (e.g., "1.45 MB").
 */
export function formatImageFileSize(bytes: number | bigint): string {
  const num = typeof bytes === "bigint" ? Number(bytes) : bytes;
  if (num < 1024) return `${num} B`;
  if (num < 1024 * 1024) return `${(num / 1024).toFixed(1)} KB`;
  if (num < 1024 * 1024 * 1024) return `${(num / (1024 * 1024)).toFixed(2)} MB`;
  return `${(num / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

/**
 * Calculates a simplified aspect ratio string (e.g., "16:9", "4:3", "1:1").
 */
export function formatAspectRatio(width: number, height: number): string {
  if (width <= 0 || height <= 0) return "";
  const gcd = (a: number, b: number): number => (b === 0 ? a : gcd(b, a % b));
  const divisor = gcd(Math.round(width), Math.round(height));
  const wRatio = Math.round(width) / divisor;
  const hRatio = Math.round(height) / divisor;

  // Standard ratio checks for near-miss floats
  const floatRatio = width / height;
  if (Math.abs(floatRatio - 16 / 9) < 0.02) return "16:9";
  if (Math.abs(floatRatio - 4 / 3) < 0.02) return "4:3";
  if (Math.abs(floatRatio - 3 / 2) < 0.02) return "3:2";
  if (Math.abs(floatRatio - 1) < 0.01) return "1:1";
  if (Math.abs(floatRatio - 9 / 16) < 0.02) return "9:16";

  if (wRatio <= 20 && hRatio <= 20) {
    return `${wRatio}:${hRatio}`;
  }

  return `${floatRatio.toFixed(2)}:1`;
}

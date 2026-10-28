import { describe, it, expect } from "vitest";
import {
  isImageFilename,
  isRawFilename,
  isExternalDelegationFilename,
  getImageFormatLabel,
  formatImageFileSize,
  formatAspectRatio,
} from "../utils/imageUtils";

describe("imageUtils", () => {
  describe("isImageFilename", () => {
    it("recognizes common raster formats", () => {
      expect(isImageFilename("photo.png")).toBe(true);
      expect(isImageFilename("photo.PNG")).toBe(true);
      expect(isImageFilename("photo.jpg")).toBe(true);
      expect(isImageFilename("photo.jpeg")).toBe(true);
      expect(isImageFilename("photo.jfif")).toBe(true);
      expect(isImageFilename("graphic.webp")).toBe(true);
      expect(isImageFilename("animation.gif")).toBe(true);
      expect(isImageFilename("modern.avif")).toBe(true);
      expect(isImageFilename("legacy.bmp")).toBe(true);
      expect(isImageFilename("favicon.ico")).toBe(true);
      expect(isImageFilename("print.tiff")).toBe(true);
      expect(isImageFilename("print.tif")).toBe(true);
    });

    it("recognizes vector SVG format", () => {
      expect(isImageFilename("vector.svg")).toBe(true);
      expect(isImageFilename("icon.SVG")).toBe(true);
    });

    it("recognizes extended formats (camera RAW, HEIC, PSD, HDR)", () => {
      expect(isImageFilename("camera.cr2")).toBe(true);
      expect(isImageFilename("camera.CR3")).toBe(true);
      expect(isImageFilename("camera.nef")).toBe(true);
      expect(isImageFilename("camera.arw")).toBe(true);
      expect(isImageFilename("camera.dng")).toBe(true);
      expect(isImageFilename("mobile.heic")).toBe(true);
      expect(isImageFilename("mobile.HEIF")).toBe(true);
      expect(isImageFilename("design.psd")).toBe(true);
      expect(isImageFilename("environment.hdr")).toBe(true);
    });

    it("rejects non-image filenames", () => {
      expect(isImageFilename("document.txt")).toBe(false);
      expect(isImageFilename("report.pdf")).toBe(false);
      expect(isImageFilename("script.ts")).toBe(false);
      expect(isImageFilename("style.css")).toBe(false);
      expect(isImageFilename("archive.tar.gz")).toBe(false);
      expect(isImageFilename("no_extension")).toBe(false);
      expect(isImageFilename("")).toBe(false);
    });
  });

  describe("format classification helpers", () => {
    it("correctly identifies camera RAW files", () => {
      expect(isRawFilename("shot.cr2")).toBe(true);
      expect(isRawFilename("shot.NEF")).toBe(true);
      expect(isRawFilename("shot.arw")).toBe(true);
      expect(isRawFilename("shot.dng")).toBe(true);
      expect(isRawFilename("shot.png")).toBe(false);
    });

    it("correctly identifies external delegation files", () => {
      expect(isExternalDelegationFilename("photo.heic")).toBe(true);
      expect(isExternalDelegationFilename("photo.heif")).toBe(true);
      expect(isExternalDelegationFilename("graphic.psd")).toBe(true);
      expect(isExternalDelegationFilename("sky.hdr")).toBe(true);
      expect(isExternalDelegationFilename("photo.jpg")).toBe(false);
    });
  });

  describe("getImageFormatLabel", () => {
    it("returns correct labels for known MIME types", () => {
      expect(getImageFormatLabel("image/png")).toBe("PNG");
      expect(getImageFormatLabel("image/jpeg")).toBe("JPEG");
      expect(getImageFormatLabel("image/svg+xml")).toBe("SVG");
      expect(getImageFormatLabel("image/webp")).toBe("WEBP");
      expect(getImageFormatLabel("image/gif")).toBe("GIF");
      expect(getImageFormatLabel("image/avif")).toBe("AVIF");
      expect(getImageFormatLabel("image/bmp")).toBe("BMP");
      expect(getImageFormatLabel("image/x-icon")).toBe("ICO");
      expect(getImageFormatLabel("image/tiff")).toBe("TIFF");
      expect(getImageFormatLabel("image/heic")).toBe("HEIC");
      expect(getImageFormatLabel("image/vnd.adobe.photoshop")).toBe("PSD");
      expect(getImageFormatLabel("image/vnd.radiance")).toBe("HDR");
      expect(getImageFormatLabel("image/x-raw")).toBe("RAW");
    });

    it("falls back to file extension or generic label", () => {
      expect(getImageFormatLabel(undefined, "photo.webp")).toBe("WEBP");
      expect(getImageFormatLabel(undefined, "unknown")).toBe("IMAGE");
    });
  });

  describe("formatImageFileSize", () => {
    it("formats bytes properly with number and bigint", () => {
      expect(formatImageFileSize(512)).toBe("512 B");
      expect(formatImageFileSize(1024)).toBe("1.0 KB");
      expect(formatImageFileSize(1536)).toBe("1.5 KB");
      expect(formatImageFileSize(1024 * 1024 * 2.5)).toBe("2.50 MB");
      expect(formatImageFileSize(BigInt(2048))).toBe("2.0 KB");
    });
  });

  describe("formatAspectRatio", () => {
    it("identifies standard aspect ratios", () => {
      expect(formatAspectRatio(1920, 1080)).toBe("16:9");
      expect(formatAspectRatio(3840, 2160)).toBe("16:9");
      expect(formatAspectRatio(800, 600)).toBe("4:3");
      expect(formatAspectRatio(1200, 800)).toBe("3:2");
      expect(formatAspectRatio(500, 500)).toBe("1:1");
      expect(formatAspectRatio(1080, 1920)).toBe("9:16");
    });

    it("handles zero or non-standard dimensions", () => {
      expect(formatAspectRatio(0, 100)).toBe("");
      expect(formatAspectRatio(100, 0)).toBe("");
      expect(formatAspectRatio(1000, 300)).toBe("10:3");
    });
  });
});

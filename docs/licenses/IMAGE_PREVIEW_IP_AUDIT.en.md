# Image Preview & Media Processing Intellectual Property (IP) & Patent Audit Record

<p align="center">
  <strong>English</strong> | <a href="IMAGE_PREVIEW_IP_AUDIT.ja.md">日本語</a> | <a href="README.md">📚 License Portal</a> | <a href="../../README.md">🏠 Root README</a>
</p>

**Document Date:** October 28, 2026  
**Status:** Completed (Pre-Audit Due Diligence Record for Phase 10 Image Preview Implementation)  
**Audience:** Comprehensive License Auditors, Legal & Compliance Reviewers, Future Maintainers  
**Target Implementation:**
* Backend: [`src-tauri/src/image_preview.rs`](../../src-tauri/src/image_preview.rs)
* Frontend: [`src/components/ImagePreview.tsx`](../../src/components/ImagePreview.tsx), [`src/utils/imageUtils.ts`](../../src/utils/imageUtils.ts)

---

## 1. Overview and Purpose

To deliver a premier user experience as a non-destructive Linux file workbench, Cuddle supports rich inline previewing not only for standard web formats (PNG, JPEG, SVG, WebP, AVIF), but also for digital camera **RAW files**, smartphone **HEIC / HEIF**, DTP/print **TIFF / PSD**, and HDR **radiance maps**.

However, multimedia and image compression algorithms present significant legal challenges, including **complex patent pools (with recurring royalty claims) and copyleft license contamination risks (LGPL/GPL)**.

This document serves as the formal due diligence record for the post-feature-freeze Comprehensive License Audit, providing verifiable technical evidence and legal rationale that Cuddle's image preview implementation **strictly preserves MIT License compatibility and completely eliminates patent infringement liabilities**.

---

## 2. Format-by-Format IP, Patent, and License Breakdown

| Format Family | Extensions | IP & Patent Landscape | Cuddle Architectural Strategy & Legal Basis | Audit Verdict |
| :--- | :--- | :--- | :--- | :---: |
| **Standard Web Raster** | `.png`, `.jpg`, `.jpeg`, `.webp`, `.gif`, `.avif`, `.bmp`, `.ico` | **Fully royalty-free or expired patents**<br>• PNG, JPEG: Open international standards.<br>• GIF: Unisys LZW patents expired worldwide in 2004.<br>• WebP: Open-sourced and royalty-free by Google.<br>• AVIF: AOMedia royalty-free format with defensive patent grant.<br>• BMP, ICO: Legacy open bitmap standards. | **WebView Native Rendering**<br>Rust reads raw bytes into memory and transfers a Base64 Data URL to WebView. No proprietary external decoding libraries are compiled or linked into the Cuddle binary. | **Clean (Compliant)** |
| **Vector Graphics** | `.svg`, `.svgz` | Open W3C XML standard. Zero patent encumbrance. | **Browser Native & XML Code View**<br>Rendered directly via WebView DOM with dual-mode XML syntax highlighting. | **Clean (Compliant)** |
| **Digital Camera RAW** | `.cr2`, `.cr3`, `.nef`, `.arw`, `.dng`, `.rw2`, `.orf`, `.pef`, `.raf` | Proprietary camera manufacturer formats based on TIFF/EP. Adobe DNG is an open specification.<br>Sensor demosaicing algorithms often carry trade secrets or patent claims. | **Embedded JPEG Extraction (Zero Demosaicing)**<br>Cuddle intentionally avoids raw sensor demosaicing. Instead, it extracts the camera manufacturer's high-resolution embedded preview JPEG stream (SOI `0xFFD8FF` / EOI `0xFFD9`).<br>Pure binary parsing with zero patent risk. Delegates to host `exiftool` if available. | **Clean (Compliant)** |
| **High Efficiency Smartphone Photos**<br>**(Critical Audit Focus)** | `.heic`, `.heif` | **Substantial Patent Pool Risks (HEVC / H.265)**<br>HEIC packages HEVC-compressed still frames inside an ISO container. Multiple patent pools (MPEG LA, HEVC Advance, Velos Media) collect royalties on hardware and software decoders.<br>Standard open-source libraries (`libheif`, `libde265`) are **LGPLv3 / GPL**, creating license contagion risks if statically linked into MIT applications. | **Dynamic CLI Delegation (Zero Bundling)**<br>**Cuddle NEVER bundles, compiles, or statically links an HEVC decoder.**<br>Cuddle inspects the host system for user-installed binaries (`magick` or `ffmpeg`). If present, it executes conversion inside an isolated Linux Landlock LSM sandbox. If absent, it displays an informative guidance message.<br>Because Cuddle does not distribute a decoder, no distributor liability attaches. | **Clean (Compliant)**<br>*(Patent risk mitigated via delegation)* |
| **DTP / Specialized Formats** | `.tiff`, `.tif`, `.psd`, `.hdr` | • TIFF: LZW patents expired globally in 2004.<br>• PSD: Adobe publishes formal format specifications; reading composite layers is unencumbered.<br>• HDR: Radiance HDR is public domain; OpenEXR is BSD-3-Clause. | **Native or Delegated Execution**<br>Read natively or converted via host tools safely. | **Clean (Compliant)** |

---

## 3. Legal Analysis of Dynamic CLI Delegation for HEIC / HEIF

### 3.1 Why Static Linking Was Rejected
Directly compiling `libheif` or an HEVC decoder into Cuddle's Rust binary was intentionally rejected due to:
1. **Direct Patent Royalty Liability:**
   Distributing an integrated executable containing an HEVC decoder subjects the project maintainers and distributors to compliance inquiries and licensing fee demands from HEVC patent pools.
2. **LGPLv3 / GPL Copyleft Contagion:**
   Satisfying LGPLv3 dynamic relinking requirements and anti-DRM clauses in standalone desktop bundles complicates distribution without exposing the entire workbench architecture to license friction.

### 3.2 Legal Protection Afforded by Dynamic CLI Delegation
* **Strict Codebase Separation (Loose Coupling):**
  Cuddle’s release binaries contain zero bytes of HEVC decoding or patent-encumbered binary code.
* **Separation of Legal Liability to Host System:**
  Conversion relies exclusively on external binaries (`/usr/bin/magick`, `/usr/bin/ffmpeg`) installed by the user via their OS package manager (e.g., Arch pacman, Ubuntu apt, Fedora dnf). Under patent law, invoking a third-party process already licensed or deployed on the user's host does not constitute unlawful distribution by Cuddle.
* **Linux Landlock LSM Kernel Isolation:**
  Sandboxed sub-processes operate with read-only access to the source file and isolated write access strictly limited to a temporary directory (`TMPDIR`), enforcing the non-destructive workbench philosophy.

---

## 4. Auditor's Checklist for Comprehensive License Audit

When conducting the final release audit at the conclusion of all feature phases, verify the following:

### ☑ Check 1: Binary Linkage & Symbol Inspection (Zero HEVC/GPL Infiltration)
Run dynamic symbol and linkage checks against the compiled release ELF binary:
```bash
# Check for embedded HEIF/HEVC decoder symbols
nm -D src-tauri/target/release/cuddle | grep -iE "heif|de265|hevc|x265"
# Expected output: (Empty / 0 matches)

# Check dynamic library dependencies
ldd src-tauri/target/release/cuddle | grep -iE "heif|de265|x265"
# Expected output: (Empty / 0 matches)
```

### ☑ Check 2: Rust Crate License Audit
Verify that all image processing and EXIF dependencies carry approved permissive licenses:
```bash
cargo deny check licenses
```
* **Key Crates:**
  * `kamadak-exif`: **BSD-2-Clause** (Permissive, commercial and redistribution friendly)
  * `base64`: **MIT / Apache-2.0** (Permissive dual-license)
  * `serde`, `serde_json`: **MIT / Apache-2.0** (Permissive dual-license)

### ☑ Check 3: Host Tool Absence Verification
Test on a minimal container environment lacking ImageMagick and FFmpeg:
1. Confirm the application does not crash when opening a `.heic` file.
2. Confirm the UI gracefully displays the localized guidance message:
   `"Previewing this format (HEIC) requires external tool (ImageMagick (magick) or FFmpeg)"`.

### ☑ Check 4: Camera RAW Embedded Preview Integrity
Verify that camera RAW images (`.cr2`, `.nef`, `.arw`) display high-resolution embedded preview JPEGs without invoking or linking any third-party raw demosaicing engines.

---

## 5. Legal Disclaimer

This document records the technical and architectural intellectual property due diligence performed by the Cuddle engineering team. It does not constitute formal legal counsel or an official patent non-infringement opinion. Patent laws and pool policies should be monitored periodically.

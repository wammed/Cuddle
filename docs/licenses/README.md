# Cuddle License & Intellectual Property (IP) Portal

<p align="center">
  <strong>English</strong> | <a href="README.ja.md">日本語</a> | <a href="../../README.md">🏠 Root README</a> | <a href="IP_COMPLIANCE.md">🛡️ Icon IP Record</a>
</p>

This document serves as the central documentation portal for third-party licenses, intellectual property (IP), patent risk mitigation, and reference records for the future comprehensive license audit in the **Cuddle (File Workbench)** project.

---

## 1. Core Principles of License & IP Governance

Cuddle is distributed under the **MIT License**, ensuring open, unrestricted usage, modification, and redistribution for developers and power users.

To maintain uncompromising legal compliance and prevent license contagion, the codebase adheres to four non-negotiable principles:

1. **Strict Permissive License Compliance:**
   * All direct and transitive dependencies compiled or statically linked into the Cuddle binary must use permissive licenses (MIT, Apache-2.0, BSD-2/3-Clause).
   * Strong copyleft licenses (GPLv2/v3, AGPL) must never be statically linked into the core binary.
2. **Elimination of Patent Pool & Royalty Risks:**
   * Proprietary or patent-encumbered codec decoders/encoders (such as HEVC / H.265 in HEIC) must never be embedded or bundled within Cuddle release binaries.
3. **Decoupled Dynamic CLI Delegation:**
   * Advanced or patent-sensitive operations (e.g., HEIC conversion, advanced metadata manipulation) must delegate execution to host-installed external utilities (ImageMagick, FFmpeg, ExifTool) inside a secure Linux Landlock LSM sandbox.
4. **Strict Bilingual Synchronization:**
   * All license and IP governance documentation must be maintained in both English and Japanese.

---

## 2. License & IP Documentation Index

| Document | Language | Description & Scope |
| :--- | :--- | :--- |
| [`IMAGE_PREVIEW_IP_AUDIT.en.md`](IMAGE_PREVIEW_IP_AUDIT.en.md) | English | **Image Preview & Media Processing IP / Patent Audit Record & Checklist (Native, RAW, HEIC, PSD, etc.)** |
| [`IMAGE_PREVIEW_IP_AUDIT.ja.md`](IMAGE_PREVIEW_IP_AUDIT.ja.md) | 日本語 | 画像プレビュー機能のライセンス・特許リスク分析と設計根拠、監査チェックリスト |
| [`IP_COMPLIANCE.md`](IP_COMPLIANCE.md) | English | Application Icon & Brand IP Compliance and Due Diligence Record |
| [`IP_COMPLIANCE.ja.md`](IP_COMPLIANCE.ja.md) | 日本語 | アプリケーションアイコンおよびブランド意匠の由来・デューデリジェンス記録 |
| [`THIRD-PARTY-LICENSES.md`](THIRD-PARTY-LICENSES.md) | English | Third-Party Dependency Licenses & Compliance Audit Record |
| [`../../LICENSE`](../../LICENSE) | English | Official MIT License terms for the Cuddle project |

---

## 3. Comprehensive License Audit Plan (Post-Feature Freeze)

Upon completion of all feature phases (Phase 14), a comprehensive formal license audit will be executed prior to final production release:

1. **Automated Rust Dependency Scan:**
   * Run `cargo-deny` across all transitive dependencies to classify licenses and detect incompatible copyleft libraries.
2. **Automated Frontend (npm) Dependency Scan:**
   * Run license extraction tools to generate complete attribution and NOTICE manifests.
3. **Binary Symbol & Linkage Verification:**
   * Inspect binary symbols using `readelf`, `nm`, and `objdump` to verify that no GPL/LGPL or patent-encumbered code was statically linked.

# Third-Party Licenses & Compliance Audit Record

**Project:** Cuddle (Safe Linux File Workbench)  
**Repository:** `wammed/Cuddle`  
**Distribution Model:** Source-Only Distribution (No precompiled binaries or installers distributed by the project)  
**Primary Project License:** [MIT License](LICENSE)

---

## 1. Distribution Policy & Compliance Boundary

Cuddle is distributed strictly as an open-source **source-code-only project**. Users build Cuddle directly from source on their Linux systems using standard toolchains (`cargo`, `pnpm`/`npm`).

- **No Binary Distribution:** The Cuddle project does not distribute precompiled release binaries, packages (`.deb`, `.rpm`, `.AppImage`), or installers.
- **Dependency Ingestion Boundary:** Build dependencies (Cargo crates, npm packages) are retrieved at build-time by the respective package managers from official registries (`crates.io`, `npmjs.org`) according to explicit locked manifests (`Cargo.lock`, `pnpm-lock.yaml`).
- **External CLI Delegation:** Media processing tools (ImageMagick, FFmpeg, ExifTool) and system search tools (plocate) are host-installed system utilities delegated via secure Landlock LSM isolation; they are never bundled into the Cuddle repository.

---

## 2. Summary of License Compliance Audit

An automated and manual license audit was conducted covering all direct, dev, and transitive dependencies across both Rust (`Cargo.lock`) and Frontend (`pnpm-lock.yaml`).

| Category | Total Checked | PASS | REVIEW | FAIL | UNKNOWN |
| :--- | ---:| ---:| ---:| ---:| ---:|
| **Cargo Dependencies (Rust)** | 486 | 480 | 6 (MPL-2.0 audited) | 0 | 0 |
| **npm Dependencies (Frontend)** | 114 | 112 | 2 (MPL-2.0 build tool) | 0 | 0 |
| **In-Tree Third-Party Assets** | 3 | 3 | 0 | 0 | 0 |

### Permissive License Categories

The vast majority of dependencies utilize permissive open-source licenses:
- **MIT / Apache-2.0 / Dual MIT/Apache-2.0:** Standard permissive licenses.
- **BSD-2-Clause / BSD-3-Clause / 0BSD:** Permissive attribution licenses.
- **ISC / Zlib / zlib-acknowledgement:** Permissive licenses.
- **Unicode-3.0:** Permissive Unicode license for normalization & ICU data.
- **Unlicense / CC0-1.0 / MIT-0:** Public domain dedication / zero-clause permissive licenses.
- **BlueOak-1.0.0:** Permissive modern open-source license (`lru-cache`).
- **CDLA-Permissive-2.0:** Permissive open agreement (`webpki-roots`).

---

## 3. Detailed Audit of Key Dependencies & Focus Areas

### 3.1 Lindera & IPADIC Morphological Dictionary

| Crate | Version | License | Upstream Source | Audit Status |
| :--- | :--- | :--- | :--- | :--- |
| `lindera` | 5.3.0 | MIT | https://github.com/lindera/lindera | PASS |
| `lindera-ipadic` | 5.3.0 | MIT | https://github.com/lindera/lindera-ipadic | PASS |
| `lindera-analysis` | 5.3.0 | MIT | https://github.com/lindera/lindera | PASS |
| `lindera-dictionary` | 5.3.0 | MIT | https://github.com/lindera/lindera | PASS |
| `lindera-tantivy` | 5.0.1 | MIT | https://github.com/lindera/lindera-tantivy | PASS |

#### IPADIC Dictionary Origin & Distribution Conditions
- **Source of Dictionary Data:** `mecab-ipadic-2.7.0` (Nara Institute of Science and Technology - NAIST, with entries originating from ICOT Free Software).
- **In-Tree Status:** The Cuddle Git repository contains **zero** IPADIC dictionary archive or binary files.
- **Build Behavior:** When building with `embed-ipadic`, `lindera-ipadic`'s `build.rs` fetches and compiles dictionary data into the binary during local user compilation.
- **Attribution & Disclaimer Notice:** In accordance with NAIST and ICOT terms, the "NO WARRANTY" disclaimer is reproduced below for transparency:

```text
===========================================================================
mecab-ipadic Notice & ICOT Free Software Condition
===========================================================================
Nara Institute of Science and Technology (NAIST), the copyright holders,
disclaims all warranties with regard to this software, including all implied
warranties of merchantability and fitness. In no event shall NAIST be liable
for any special, indirect or consequential damages or any damages whatsoever
resulting from loss of use, data or profits.

A large portion of the dictionary entries originate from ICOT Free Software.
Conditions for ICOT Free Software:
Each User may freely distribute the Program, whether in its original form or
modified, PROVIDED that the provisions of Section 3 ("NO WARRANTY") will ALWAYS
appear on, or be attached to, the Program:

NO WARRANTY:
The program was produced on an experimental basis in the course of the
research and development conducted during the project and is provided
to users as so produced on an experimental basis. Accordingly, the
program is provided without any warranty whatsoever, whether express,
implied, statutory or otherwise.
===========================================================================
```

---

### 3.2 Mozilla Public License 2.0 (MPL-2.0) Individual Review

MPL-2.0 is a file-level weak copyleft license that explicitly permits combining unmodified files into a "Larger Work" under a different license (e.g. MIT, Section 3.3).

| Package / Crate | Type | Version | License | Role / Boundary | Review Verdict |
| :--- | :--- | :--- | :--- | :--- | :--- |
| `cssparser` | Cargo (transitive) | 0.37.0 | MPL-2.0 | Used by `dom_query` inside Tauri HTML parsing | PASS (Unmodified utility, isolated) |
| `cssparser-macros` | Cargo (transitive) | 0.7.1 | MPL-2.0 | Proc-macro for `cssparser` | PASS (Compile-time macro, isolated) |
| `dtoa-short` | Cargo (transitive) | 0.3.5 | MPL-2.0 | Fast float-to-string conversion in `cssparser` | PASS (Unmodified utility, isolated) |
| `selectors` | Cargo (transitive) | 0.38.0 | MPL-2.0 | CSS selector engine for `dom_query` | PASS (Unmodified utility, isolated) |
| `option-ext` | Cargo (transitive) | 0.2.0 | MPL-2.0 | Extension trait for `Option` in `dirs-sys` | PASS (Unmodified utility, isolated) |
| `htmlescape` | Cargo (transitive) | 0.3.1 | MPL-2.0 | HTML entity escaping in `tantivy` | PASS (Unmodified utility, isolated) |
| `lightningcss` | npm (devDependencies) | 1.33.0 | MPL-2.0 | CSS bundler in Vite build toolchain | PASS (Build-time only, never bundled in runtime) |
| `lightningcss-linux-x64-gnu` | npm (devDependencies) | 1.33.0 | MPL-2.0 | Platform native binary for `lightningcss` | PASS (Build-time only, never bundled in runtime) |

Under Cuddle's source-only distribution model, Cuddle does not modify any source code of these MPL-2.0 libraries. Each crate/package is independently resolved and compiled from upstream.

---

### 3.3 Disjunctive Tri-License Review: `r-efi`

| Crate | Version | License Expression | Selected Option |
| :--- | :--- | :--- | :--- |
| `r-efi` | 5.3.0, 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | **MIT** |

`r-efi` (UEFI reference implementation in Rust) is pulled transitively by `getrandom` for non-Linux platform definitions. Because `r-efi` is tri-licensed with disjunctive `OR`, Cuddle selects the **MIT License**. No LGPL-2.1 obligations are incurred.

---

### 3.4 In-Tree Third-Party Scaffolding Assets

| Asset Path | Origin | License | Note |
| :--- | :--- | :--- | :--- |
| `public/vite.svg` | Vite Scaffold | MIT | Default Vite template logo, not rendered in app UI |
| `public/tauri.svg` | Tauri Scaffold | MIT / Apache-2.0 | Default Tauri template logo, not rendered in app UI |
| `src/assets/react.svg` | React Scaffold | MIT / CC0-1.0 | Default React template logo, not rendered in app UI |
| `public/Cuddle-Icon.svg` | Cuddle Project | MIT | Project's original icon (see [IP_COMPLIANCE.md](IP_COMPLIANCE.md)) |

---

## 4. Continuous License Verification

To prevent accidental introduction of copyleft or unauthorized licenses, two automated license gates are integrated into the continuous integration (CI) workflow:

1. **Rust / Cargo (`deny.toml`):**
   ```bash
   cargo deny --manifest-path src-tauri/Cargo.toml check licenses
   ```
2. **Frontend / npm (`scripts/check-frontend-licenses.mjs`):**
   ```bash
   npm run check:licenses
   ```


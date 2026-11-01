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
| **In-Tree Third-Party Assets** | 0 | 0 | 0 | 0 | 0 |

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
- **Source of Dictionary Data:** `mecab-ipadic-2.7.0-20070801` (Nara Institute of Science and Technology - NAIST, with entries originating from ICOT Free Software).
- **Target Crate:** `lindera-ipadic` version `5.3.0` (pinned via `Cargo.lock`).
- **Upstream Source & Reference:** [lindera-ipadic/NOTICE.txt](https://github.com/lindera/lindera/blob/21de7a4a38ac632d053bda645f5dcf01cd67bbdd/lindera-ipadic/NOTICE.txt).
- **In-Tree Status:** The Cuddle Git repository contains **zero** IPADIC dictionary archive or binary files.
- **Build Behavior:** When building with `embed-ipadic`, `lindera-ipadic`'s `build.rs` fetches and compiles dictionary data into the binary during local user compilation.
- **Distribution Boundary:** Cuddle is distributed exclusively under a **source-code-only distribution model** (no pre-built binaries, RPM, Deb, or AppImage packages are distributed by the project).
- **Attribution & Disclaimer Notice (Method A: Verbatim Upstream Notice):** In accordance with NAIST and ICOT terms, the complete and unabridged upstream `NOTICE.txt` is reproduced verbatim below:

```text
===========================================================================
Lindera Morphological Analyzer
===========================================================================

This software includes a binary and/or source version of data from

  mecab-ipadic-2.7.0-20070801

which can be obtained from

  http://jaist.dl.sourceforge.net/project/mecab/mecab-ipadic/2.7.0-20070801/mecab-ipadic-2.7.0-20070801.tar.gz

===========================================================================
mecab-ipadic-2.7.0-20070801 Notice
===========================================================================

Nara Institute of Science and Technology (NAIST),
the copyright holders, disclaims all warranties with regard to this
software, including all implied warranties of merchantability and
fitness, in no event shall NAIST be liable for
any special, indirect or consequential damages or any damages
whatsoever resulting from loss of use, data or profits, whether in an
action of contract, negligence or other tortuous action, arising out
of or in connection with the use or performance of this software.

A large portion of the dictionary entries
originate from ICOT Free Software.  The following conditions for ICOT
Free Software applies to the current dictionary as well.

Each User may also freely distribute the Program, whether in its
original form or modified, to any third party or parties, PROVIDED
that the provisions of Section 3 ("NO WARRANTY") will ALWAYS appear
on, or be attached to, the Program, which is distributed substantially
in the same form as set out herein and that such intended
distribution, if actually made, will neither violate or otherwise
contravene any of the laws and regulations of the countries having
jurisdiction over the User or the intended distribution itself.

NO WARRANTY

The program was produced on an experimental basis in the course of the
research and development conducted during the project and is provided
to users as so produced on an experimental basis.  Accordingly, the
program is provided without any warranty whatsoever, whether express,
implied, statutory or otherwise.  The term "warranty" used herein
includes, but is not limited to, any warranty of the quality,
performance, merchantability and fitness for a particular purpose of
the program and the nonexistence of any infringement or violation of
any right of any third party.

Each user of the program will agree and understand, and be deemed to
have agreed and understood, that there is no warranty whatsoever for
the program and, accordingly, the entire risk arising from or
otherwise connected with the program is assumed by the user.

Therefore, neither ICOT, the copyright holder, or any other
organization that participated in or was otherwise related to the
development of the program and their respective officials, directors,
officers and other employees shall be held liable for any and all
damages, including, without limitation, general, special, incidental
and consequential damages, arising out of or otherwise in connection
with the use or inability to use the program or any product, material
or result produced or otherwise obtained by using the program,
regardless of whether they have been advised of, or otherwise had
knowledge of, the possibility of such damages at any time during the
project or thereafter.  Each user will be deemed to have agreed to the
foregoing by his or her commencement of use of the program.  The term
"use" as used herein includes, but is not limited to, the use,
modification, copying and distribution of the program and the
production of secondary products from the program.

In the case where the program, whether in its original form or
modified, was distributed or delivered to or received by a user from
any person, organization or entity other than ICOT, unless it makes or
grants independently of ICOT any specific warranty to the user in
writing, such person, organization or entity, will also be exempted
from and not be held liable to the user for any such damages as noted
above as far as the program is concerned.
===========================================================================
```

---

### 3.2 Mozilla Public License 2.0 (MPL-2.0) & Frontend Dependencies Review

#### A. MPL-2.0 Evaluation & Non-Infection Boundary
MPL-2.0 is a file-level weak copyleft license. Under Sections 1.4, 3.1, and 3.3 of MPL-2.0:
- Copyleft obligations apply strictly to "Covered Software" (the original MPL-licensed source code files and direct modifications to those files).
- Unmodified MPL-licensed software may be combined with independent software into a "Larger Work" without subjecting the larger work or independent modules to MPL-2.0 terms.
- In Cuddle, no MPL-2.0 source code is copied, modified, or incorporated into Cuddle's codebase.
- Under Cuddle's **source-only distribution model**, Cuddle distributes only its own original source code under the MIT License.

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

#### B. Frontend Dependencies Classification
1. **Direct Runtime Dependencies (`dependencies`):**
   - `@tanstack/react-virtual@^3.14.22`: MIT
   - `@tauri-apps/api@^2`: Apache-2.0 OR MIT
   - `@tauri-apps/plugin-opener@^2`: Apache-2.0 OR MIT
   - `i18next@^26.5.6`: MIT
   - `lucide-react@^1.61.0`: ISC
   - `react@^19.1.0`: MIT
   - `react-dom@^19.1.0`: MIT
   - `react-i18next@^17.1.3`: MIT
   - `react-resizable-panels@^4.14.8`: MIT
   *All direct runtime dependencies are 100% permissively licensed (MIT, Apache-2.0, ISC).*
2. **Development & Build-Time Dependencies (`devDependencies`):**
   - Direct: `@tauri-apps/cli` (Apache-2.0/MIT), `@testing-library/jest-dom` (MIT), `@testing-library/react` (MIT), `@types/react` (MIT), `@types/react-dom` (MIT), `@vitejs/plugin-react` (MIT), `jsdom` (MIT), `typescript` (Apache-2.0), `vite` (MIT), `vitest` (MIT).
   - Indirect: `lightningcss@1.33.0` and its Linux native companion `lightningcss-linux-x64-gnu@1.33.0` (MPL-2.0) are resolved as transitive build-time dependencies of `vite@8.3.5` for CSS bundling and minification. They are never executed in or shipped with the runtime frontend client. Cuddle's source code remains entirely unaffected under the MIT License.

---

### 3.3 Disjunctive Tri-License Review: `r-efi`

| Crate | Version | License Expression | Selected Option |
| :--- | :--- | :--- | :--- |
| `r-efi` | 5.3.0, 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | **MIT** |

`r-efi` (UEFI reference implementation in Rust) is pulled transitively by `getrandom` for non-Linux platform definitions. Because `r-efi` is tri-licensed with disjunctive `OR`, Cuddle selects the **MIT License**. No LGPL-2.1 obligations are incurred.

---

### 3.4 In-Tree Third-Party Scaffolding Assets (Removed)

The original template scaffold generated by `create-tauri-app` included default logos for upstream frameworks (`public/tauri.svg`, `public/vite.svg`, and `src/assets/react.svg`).

- **Usage in Cuddle:** None. These files were completely unreferenced in the application source code, UI components, HTML, scripts, and documentation.
- **Licensing & Trademark Assessment:**
  - `public/tauri.svg`: The Tauri code repository is dual-licensed under MIT or Apache-2.0, but the Tauri logo and branding are protected trademarks subject to Tauri's Trademark Guidelines and brand terms (not standard permissive code redistribution without attribution/restrictions). Stating "MIT / Apache-2.0" conflated code licensing with trademark policy.
  - `public/vite.svg` & `src/assets/react.svg`: Framework branding marks belonging to their respective owners (Evan You / Vite contributors and Meta Platforms, Inc.).
- **Remediation:** To eliminate any trademark ambiguity, licensing misattribution, or dead-code footprint, all three unused scaffolding logo files have been completely removed from the repository. Cuddle contains **zero** in-tree third-party graphics or brand assets.

---

### 3.5 Cuddle Project Original Brand Asset

| Asset Path | Origin | License | Description / Status |
| :--- | :--- | :--- | :--- |
| `public/Cuddle-Icon.svg` | Cuddle Project | MIT License | Official application icon (see [IP_COMPLIANCE.md](IP_COMPLIANCE.md)) |

`public/Cuddle-Icon.svg` is **not a third-party asset**. It is Cuddle's own original application brand asset, created and iteratively refined using Google Gemini under the project's consistent visual identity (protective C-shaped cradle, document, security key, and glowing shield). Its full design history, provenance records, and similarity reviews are maintained in [`IP_COMPLIANCE.md`](IP_COMPLIANCE.md). It is governed by Cuddle's primary [MIT License](LICENSE).

---

## 4. License Verification

Rust / Cargo dependencies are continuously checked against allowed licenses in CI using `cargo-deny` with [`deny.toml`](deny.toml):

```bash
cargo deny --manifest-path src-tauri/Cargo.toml check licenses
```

Frontend / npm dependencies are documented above and audited against Cuddle's source-only distribution model. All runtime frontend packages utilize permissive licenses (MIT, Apache-2.0, BSD, ISC), with `lightningcss` (MPL-2.0) confined strictly to build-time tooling (`devDependencies`).


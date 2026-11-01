# Third-Party Licenses & Compliance Audit Record

**Project:** Cuddle (Safe Linux File Workbench)  
**Repository:** `wammed/Cuddle`  
**Distribution Model:** Source-Only Distribution (No precompiled binaries or installers distributed by the project)  
**Primary Project License:** [MIT License](../../LICENSE)

---

## 1. Distribution Policy & Compliance Boundary

Cuddle is distributed strictly as an open-source **source-code-only project**. Users build Cuddle directly from source on their Linux systems using standard toolchains (`cargo`, `pnpm`/`npm`).

- **No Binary Distribution:** The Cuddle project does not distribute precompiled release binaries, packages (`.deb`, `.rpm`, `.AppImage`), or installers.
- **Dependency Ingestion Boundary:** Build dependencies (Cargo crates, npm packages) are retrieved at build-time by the respective package managers from official registries (`crates.io`, `registry.npmjs.org`) according to explicit locked manifests (`Cargo.lock`, `pnpm-lock.yaml`).
- **External CLI Delegation:** Media processing tools (ImageMagick, FFmpeg, ExifTool) and system search tools (plocate) are host-installed system utilities delegated via secure Landlock LSM isolation; they are never bundled into the Cuddle repository.

---

## 2. Dependency Scope & Compliance Policy

Cuddle tracks third-party dependencies through locked manifests (`Cargo.lock`, `pnpm-lock.yaml`) and verifies licenses against its source-only distribution model:

- **Rust / Cargo Dependencies:** Managed via `src-tauri/Cargo.lock`. Automated continuous license verification is enforced in CI via `cargo-deny` with [`deny.toml`](../../deny.toml), which permits explicitly defined permissive licenses and enforces individually audited exception entries for specific transitive MPL-2.0 utility crates.
- **Frontend / npm Dependencies:** Defined in `package.json` and resolved via `pnpm-lock.yaml`. Direct runtime dependencies are specified under `dependencies`, while development, testing, and bundling tools are isolated under `devDependencies`.
- **In-Tree Third-Party Graphic Assets:** **0**. All third-party template scaffolding logos have been removed from the repository. The sole graphic brand asset in-tree is Cuddle's original icon (`public/Cuddle-Icon.svg`), governed by Cuddle's MIT License (see [`IP_COMPLIANCE.md`](IP_COMPLIANCE.md)).

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

### 3.2 Mozilla Public License 2.0 (MPL-2.0) Dependency Review

#### A. MPL-2.0 Licensing Principles & Scope
The Mozilla Public License 2.0 (MPL-2.0) is a file-level weak copyleft license:
1. **Source Code Distribution Requirements (Section 3.1):** Any modifications to Covered Software files must remain under MPL-2.0 when distributed in source code form. Distribution of Covered Software in source code form requires providing recipients access to the source code under MPL-2.0 terms.
2. **Combination in a Larger Work (Section 3.3):** Section 3.3 explicitly permits combining Covered Software with files governed by different, independent licenses (e.g. MIT) to create a "Larger Work," provided the license terms for the independent files are respected and the MPL-2.0 terms continue to apply to the Covered Software files. MPL-2.0 does not automatically propagate to independent source files.
3. **Executable Distribution Requirements (Section 3.2):** If Covered Software is distributed in executable or binary form, Section 3.2 requires that the distributor inform recipients how they may obtain the source code of the Covered Software and provide a copy of the license.
4. **Cuddle Source-Only Distribution & Ingestion Boundary:**
   - Cuddle is maintained and distributed strictly as a source-code-only project; the project does not distribute precompiled release binaries, packages, or installers.
   - Crucially, Cuddle's repository contains **zero** in-tree, bundled, or vendored source code files from MPL-2.0 dependencies. Dependencies are resolved separately by the package managers (`cargo`, `pnpm`) from their configured package registries (normally crates.io for Rust and the npm registry for frontend packages) during local user compilation.
   - Because Cuddle distributes only its own original source files and does not redistribute or modify the Covered Software files themselves, Cuddle's repository publication does not constitute redistributing Covered Software under Section 3.1.
5. **Project Impact & Assessment:**
   - The presence of MPL-2.0 dependencies in the external dependency graph does not alter Cuddle's own MIT License.
   - Cuddle's own source tree contains zero copied or modified MPL-2.0 source code files.
   - If the project's distribution model were to expand in the future—either by vendoring/bundling third-party source files directly into the repository (triggering Section 3.1 source distribution obligations) or by distributing precompiled binaries (triggering Section 3.2 source notice obligations)—those respective MPL-2.0 requirements must be re-evaluated and fulfilled at that time.

#### B. Classification of MPL-2.0 Dependencies in Cuddle

| Package / Crate | Ecosystem | Version | Role in Project | Execution Phase |
| :--- | :--- | :--- | :--- | :--- |
| `cssparser` | Cargo (transitive) | 0.37.0 | CSS parsing engine in `dom_query` (used in Tauri HTML handling) | Runtime library / Build utility |
| `cssparser-macros` | Cargo (transitive) | 0.7.1 | Procedural macro for `cssparser` | Compile-time macro only |
| `dtoa-short` | Cargo (transitive) | 0.3.5 | Fast float-to-string conversion in `cssparser` | Runtime library utility |
| `selectors` | Cargo (transitive) | 0.38.0 | CSS selector engine in `dom_query` | Runtime library utility |
| `option-ext` | Cargo (transitive) | 0.2.0 | Extension trait for `Option` in `dirs-sys` (directory resolution) | Runtime library utility |
| `htmlescape` | Cargo (transitive) | 0.3.1 | HTML entity escaping utility in `tantivy` (search highlighting) | Runtime library utility |
| `lightningcss` | npm (transitive via Vite) | 1.33.0 | CSS bundler and transformer in Vite build toolchain | Build-time tooling only (`devDependencies`) |
| `lightningcss-linux-x64-gnu` | npm (transitive via Vite) | 1.33.0 | Native Linux x86_64 prebuilt binary for `lightningcss` | Build-time tooling only (`devDependencies`) |

- **Rust MPL-2.0 Crates:** These are unmodified third-party libraries resolved from `crates.io`. They are not all strictly build-time tools: while `cssparser-macros` is compile-time only, crates like `option-ext` (directory path resolution in `dirs-sys`), `htmlescape` (HTML entity escaping in `tantivy`), `cssparser` and `selectors` (CSS parsing and selection in `dom_query`), and `dtoa-short` (fast float-to-string formatting helper in `cssparser`) provide utility functions that are linked into the binary at compile time. Because Cuddle does not modify their source code, does not vendor them in-tree, and distributes only its own source code, these crates are permitted via individually audited exceptions in `deny.toml`.
- **Frontend MPL-2.0 Packages:** Both `lightningcss` and its Linux companion are transitive dependencies of `vite@8.3.5` under `devDependencies`. They act solely as build-time preprocessing and minification tools during `npm run build`. They are never executed in or shipped as part of the frontend client runtime assets.

---

### 3.3 Frontend Dependencies Classification

Cuddle distinguishes between direct and transitive dependencies, as well as runtime and build-time roles:

#### A. Direct Runtime Dependencies (`dependencies` in `package.json`)
The following 9 packages are directly declared as runtime dependencies:
- `@tanstack/react-virtual` (`^3.14.22`): MIT
- `@tauri-apps/api` (`^2`): Apache-2.0 OR MIT
- `@tauri-apps/plugin-opener` (`^2`): Apache-2.0 OR MIT
- `i18next` (`^26.5.6`): MIT
- `lucide-react` (`^1.61.0`): ISC
- `react` (`^19.1.0`): MIT
- `react-dom` (`^19.1.0`): MIT
- `react-i18next` (`^17.1.3`): MIT
- `react-resizable-panels` (`^4.14.8`): MIT

Each direct runtime package is confirmed to be licensed under permissive open-source terms (MIT, Apache-2.0, or ISC). This permissive status applies strictly to these declared direct dependencies and is not extrapolated as an unverified blanket statement covering the entire transitive tree.

#### B. Direct Development & Build-Time Dependencies (`devDependencies` in `package.json`)
Declared direct development dependencies include:
- `@tauri-apps/cli` (`^2`): Apache-2.0 OR MIT
- `@testing-library/jest-dom` (`^7.0.1`): MIT
- `@testing-library/react` (`^16.3.3`): MIT
- `@types/react` (`^19.1.8`): MIT
- `@types/react-dom` (`^19.1.6`): MIT
- `@vitejs/plugin-react` (`^6.0.2`): MIT
- `jsdom` (`^30.1.3`): MIT
- `typescript` (`~6.0.3`): Apache-2.0
- `vite` (`^8.0.16`): MIT
- `vitest` (`^5.0.5`): MIT

These packages are used solely for local development, TypeScript typechecking, automated testing with Vitest, and asset bundling with Vite.

#### C. Transitive Dependencies (`pnpm-lock.yaml`)
Transitive dependencies are resolved and pinned by `pnpm-lock.yaml`. Notably, `vite` pulls `lightningcss@1.33.0` and `lightningcss-linux-x64-gnu@1.33.0` (MPL-2.0) as build-time CSS processors, as evaluated above. Cuddle does not assume or assert unverified license claims for transitive packages without specific investigation; under the source-only distribution model, users retrieve locked packages directly from the official registry.

---

### 3.4 Disjunctive Tri-License Review: `r-efi`

| Crate | Version | License Expression | Selected Option |
| :--- | :--- | :--- | :--- |
| `r-efi` | 5.3.0, 6.0.0 | `MIT OR Apache-2.0 OR LGPL-2.1-or-later` | **MIT** |

`r-efi` (UEFI reference implementation in Rust) is pulled transitively by `getrandom` for non-Linux platform definitions. Because `r-efi` is tri-licensed with disjunctive `OR`, Cuddle selects the **MIT License**. No LGPL-2.1 obligations are incurred.

---

### 3.5 In-Tree Third-Party Scaffolding Assets (Removed)

The original template scaffold generated by `create-tauri-app` included default logos for upstream frameworks (`public/tauri.svg`, `public/vite.svg`, and `src/assets/react.svg`).

- **Usage in Cuddle:** None. These files were completely unreferenced in the application source code, UI components, HTML, scripts, and documentation.
- **Licensing & Trademark Assessment:**
  - `public/tauri.svg`: The Tauri code repository is dual-licensed under MIT or Apache-2.0, but the Tauri logo and branding are protected trademarks subject to Tauri's Trademark Guidelines and brand terms (not standard permissive code redistribution without attribution/restrictions). Stating "MIT / Apache-2.0" conflated code licensing with trademark policy.
  - `public/vite.svg` & `src/assets/react.svg`: Framework branding marks belonging to their respective owners (Evan You / Vite contributors and Meta Platforms, Inc.).
- **Remediation:** To eliminate any trademark ambiguity, licensing misattribution, or dead-code footprint, all three unused scaffolding logo files have been completely removed from the repository. Cuddle contains **zero** in-tree third-party graphics or brand assets.

---

### 3.6 Cuddle Project Original Brand Asset

| Asset Path | Origin | License | Description / Status |
| :--- | :--- | :--- | :--- |
| `public/Cuddle-Icon.svg` | Cuddle Project | MIT License | Official application icon (see [IP_COMPLIANCE.md](IP_COMPLIANCE.md)) |

`public/Cuddle-Icon.svg` is **not a third-party asset**. It is Cuddle's own original application brand asset, created and iteratively refined using Google Gemini under the project's consistent visual identity (protective C-shaped cradle, document, security key, and glowing shield). Its full design history, provenance records, and similarity reviews are maintained in [`IP_COMPLIANCE.md`](IP_COMPLIANCE.md). It is governed by Cuddle's primary [MIT License](../../LICENSE).

---

## 4. Continuous License Verification

- **Rust / Cargo:** Continuously enforced in CI via `cargo-deny` with [`deny.toml`](../../deny.toml):
  ```bash
  cargo deny --manifest-path src-tauri/Cargo.toml check licenses
  cargo deny --manifest-path src-tauri/Cargo.toml check
  ```
  `cargo-deny` automatically checks all direct and transitive Rust dependencies against allowed license lists, unmaintained advisories, duplicate bans, and verified exception entries.
- **Frontend / npm:** Direct dependencies are maintained in `package.json` and locked in `pnpm-lock.yaml`. Under the source-only distribution model, dependencies are resolved directly from official package registries by the user's package manager.


# FileWorkbench (Cuddle) 開発引き継ぎ書

<p align="center">
  <a href="FileWorkbench-Handover.en.md">English</a> | <strong>日本語</strong>
</p>

**文書作成日：** 2026-10-24  
**ステータス：** 設計・ロードマップ策定完了、Phase 1.1 着手準備完了  
**対象：** 開発・設計・テスト担当者、次期作業セッション担当  

---

## 1. プロジェクト概要

* **プロジェクト名：** FileWorkbench（リポジトリ: `Cuddle`）
* **目的：** 独自のブランドとビジュアルを持つ、Linux 向けの高信頼ファイル管理・作業環境・検索アプリ。
* **基本方針・コア思想：**
  1. **原本保護の絶対性：** 原本を直接編集・変更せず、独立した作業コピー（`workspace-temp/`）に対してのみ操作・変換を行う。
  2. **確定操作と原本反映の完全分離：** パイプライン実行や一括リネームなどの内部で原本を書き換えず、成果物確定後にユーザーの独立した明示操作としてのみ原本へ反映する。
  3. **Landlock LSM によるカーネルレベル隔離：** 外部ツール（ImageMagick, ffmpeg, cwebp等）実行時は、Rust 起動直前に Linux Landlock ルールセットを適用し、原本や `$HOME` への書き込みを物理的に遮断する。
  4. **決定論的ロールバック（RollbackAll）：** 一括操作において 1 件でもエラーが発生した場合は全体を失敗とし、開始前の初期状態へ完全復元する。
  5. **ドキュメント類の完全バイリンガル化（必須）：** すべての設計書・仕様書・README・引き継ぎ書・ロードマップ等のドキュメント類は、**同内容の英語版と日本語版の2通を必ず作成し、常に同期維持**する。
  6. **アプリ内表示の完全バイリンガル・動的切替（必須）：** アプリ内のメニュー、警告、メッセージ、ダイアログなどすべての UI 表示を**日本語と英語のバイリンガル対応**とし、アプリ内の設定から再起動不要で即時切り替え可能とする。
* **開発・検証対象環境：** CachyOS + COSMIC DE（Wayland, HiDPI）※初期版実機保証対象

---

## 2. 重要ドキュメント一覧

| ドキュメント (日本語版) | ドキュメント (英語版) | 説明 |
| :--- | :--- | :--- |
| [`FileWorkbench-Design-ver0.9.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.ja.md) | [`FileWorkbench-Design-ver0.9.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.en.md) | 統合基本設計書（残課題解決・バイリンガル仕様確定版） |
| [`FileWorkbench-Handover.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.ja.md) | [`FileWorkbench-Handover.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.en.md) | 開発引き継ぎ書（本ドキュメント） |
| [`roadmap.ja.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.ja.md) | [`roadmap.en.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.en.md) | 詳細実装ロードマップ（全9フェーズ・28サブフェーズ） |
| [`IP_COMPLIANCE.ja.md`](file:///home/susie/git/wammed/Cuddle/IP_COMPLIANCE.ja.md) | [`IP_COMPLIANCE.md`](file:///home/susie/git/wammed/Cuddle/IP_COMPLIANCE.md) | アイコンデザイン・知的財産コンプライアンス記録 |

---

## 3. 確定済み技術スタック & アーキテクチャ

| 領域 | 採用技術 | 確定方針・詳細仕様 |
| :--- | :--- | :--- |
| **UI 基盤** | **Tauri 2 + React + TypeScript** | バックエンドが Rust ネイティブのため、OS システムコール（Landlock、inode、POSIX権限）を直接制御可能 |
| **国際化 (i18n)** | **`i18next` + `react-i18next`** | 日本語・英語の完全バイリンガル。設定画面で即時切替（再起動不要）。エラーはコードと引数を返しUIで多言語展開 |
| **UI レンダリング** | **`@tanstack/react-virtual`** | 最大 10,000 件の一括操作一覧・プレビュー・検索結果を 60fps で描画する仮想スクロール |
| **メタデータ・履歴** | **SQLite (WAL モード)** | 通常運用は `PRAGMA synchronous = NORMAL;`、原本反映確定・バッチコミットなど重要境界でのみ `PRAGMA wal_checkpoint(TRUNCATE);` を実行 |
| **全文検索** | **Tantivy + `lindera-tantivy`** | 日本語形態素解析（IPADIC, Normal）＋ n-gram 併用による高精度・高再現率検索 |
| **ファイル名検索** | **plocate / 直接走査** | 通常時は plocate を活用し、利用不能時は `walkdir` による直接走査へ自動フォールバック |
| **プロセス分離** | **Linux Landlock (LSM)** | 非特権アクセス制限。TOML プリセットで `allowed_read_paths` を宣言可能。個別隔離 `TMPDIR`（`workspace-temp/proc-tmp-<pid>/`）を注入 |
| **本文抽出** | **段階導入 & 分離ワーカー** | プレーンテキスト/MD/HTMLは直接抽出。PDF/Office はメモリ制限クォータ（256 MiB）付き分離ワーカープロセスで実行し本体クラッシュ・OOM を防止 |
| **作業コピー領域** | **同一ボリューム優先配置** | 原本と同一マウントポイント配下の作業領域（例: `<原本ルート>/.fileworkbench-temp/`）を優先利用し、原子的 `rename(2)` と Reflink (CoW) を最大化。原本サイズ × 1.5倍の空き容量事前チェックを義務化 |
| **非 UTF-8 ファイル名** | **二重キー管理仕様** | Rust 内部は `PathBuf`（`[u8]`）を一貫保持。IPC / JSON 境界では Base64 文字列（`raw_base64`）と表示文字列（`display_name`）を併載し、固定 `item_id`（UUID/inode）で照合 |
| **一括操作** | **`RollbackAll` & ジャーナル** | `BatchPlan` / `BatchStatus` によるトランザクション管理。故障注入テストマトリクス（B-01〜B-15）を自動テストで全件パス |

---

## 4. 現在の開発ステータスと進捗

* [x] **基本設計フェーズ：** v0.5〜v0.8 の統合および残課題解決、バイリンガル仕様を反映し `ver 0.9` として設計確定（日英両言語）。
* [x] **ロードマップ策定フェーズ：** 全9フェーズ・28サブフェーズに細分化、テストゲートとユーザー意思確認ゲート、i18n要件を定義してユーザー承認取得済み（日英両言語）。
* [ ] **実装フェーズ：** **Phase 1.1（プロジェクト骨格 & 開発基盤構築）の着手待ち**。

---

## 5. フェーズ運用ルール & 進行プロトコル（厳守事項）

今後の実装進行時は、必ず以下のプロトコルを順守すること。

1. **フェーズ開始前意思決定ゲート：**  
   各サブフェーズ（Phase X.Y）の実装に入る前に、該当フェーズで確定すべき技術選定・仕様・選択肢をユーザーに提示し、合意・意思決定を得てから実装を開始する（※無断着手禁止）。
2. **完全バイリンガル実装の徹底：**  
   * 実装するすべての UI、メッセージ、エラーテキストは `locales/en/*.json` および `locales/ja/*.json` の両方に定義する。ハードコードされた単一言語テキストを一切残さない。
   * 作成・更新するすべてのドキュメント（README、設計書、タスクシート等）は同内容の日英両言語で作成する。
3. **次フェーズ移行基準（Exit Criteria）：**  
   フェーズ完了時は、以下をすべて満たすことを必須とする。
   * 定義された**自動テスト（ユニットテスト・結合テスト）の 100% パス**（Cargo / Vitest）。
   * 未翻訳キー（欠損キー）がゼロであることのバリデーションパス。
   * 定義された**手動テスト項目（実機・CLI・UI・言語切替動作）の全チェック完了**。
   * 未解決ブロッカーがゼロであること。
   * テスト結果レポートをユーザーに提示し、次フェーズへの移行承認を得ること。

---

## 6. 次期セッション着手手順（Phase 1.1 のタスク）

Phase 1.1 における推奨合意方針および初期コマンド手順：

### 6.1 Phase 1.1 で確定した方針
* **パッケージマネージャー：** `pnpm`
* **Rust Edition：** `2021`（安定性を優先）
* **ディレクトリ構成：**
  * バックエンド：`/home/susie/git/wammed/Cuddle/src-tauri/`
  * フロントエンド：`/home/susie/git/wammed/Cuddle/src/`
  * 多言語辞書：`/home/susie/git/wammed/Cuddle/src/locales/{en,ja}/`

### 6.2 実行する初期化手順
1. **Tauri 2 プロジェクト初期化：**
   ```bash
   pnpm create tauri-app . --template react-ts --manager pnpm
   ```
2. **依存クレート・パッケージの追加：**
   * Rust: `serde`, `serde_json`, `tokio`, `thiserror`, `anyhow`
   * Frontend: `i18next`, `react-i18next`, `@tanstack/react-virtual`, `lucide-react`
3. **i18n 基盤整備：**
   * `src/i18n.ts` 初期化設定、`src/locales/en/translation.json`, `src/locales/ja/translation.json`
4. **リント・フォーマッタ整備：**
   * `cargo clippy`, `cargo fmt`, `eslint`, `prettier`
5. **テスト実行：**
   * `cargo test`
   * `pnpm test`（未翻訳キーチェック含む）
   * CachyOS 上での `pnpm tauri dev` 起動および英語/日本語切替確認

---

## 7. 開発上の重要制約（Guardrails）

* **root 権限実行の禁止：** アプリは常に非特権一般ユーザー権限で動作させる（権限昇格 `sudo`/`pkexec` は行わない）。
* **原本保護の徹底：** 原本ファイルに対する直接編集・インプレース更新は行わない。すべての変更は `workspace-temp/` 内の作業コピーに対して行い、明示的な反映操作でのみ置換する。
* **安全警告の非表示化禁止：** 破壊的操作、権限不整合、原本競合に関する重大警告は、ユーザー設定でオフにできないようにハードコードする。
* **任意シェル文字列実行の排除：** コマンド実行時は `sh -c` や `bash -c` を一切使用せず、引数配列による直接プロセス起動（Landlock 適用下）を行う。
* **単一言語ハードコードの排除：** UI 上に直接日本語または英語文字列をハードコードせず、必ず `t('key')` 経由で取得する。

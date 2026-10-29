# FileWorkbench (Cuddle) 開発引き継ぎ書

<p align="center">
  <a href="FileWorkbench-Handover.en.md">English</a> | <strong>日本語</strong>
</p>

**文書作成日：** 2026-10-29  
**ステータス：** 全フェーズ（Phase 1〜11）実装・実機手動テスト承認完了、ソース公開配布準備完了（MIT License / README / BUILD / CI Actions 整備完了）、Phase 12（パイプライン外部連携柔軟性拡張：allowed_read_paths、入力方式 batch/stdin、型付きプレースホルダー、FreeDesktop デスクトップ通知抽象化）実装・自動テスト合格完了（実機手動テスト実施中）  
**対象：** 開発・設計・テスト担当者、次期作業セッション担当  

---

## 1. プロジェクト概要

* **プロジェクト名：** Cuddle
* **説明：** File Workbench
* **公式アイコン：** `Cuddle/images/Cuddle-Icon.svg`
* **README用バナー：** `Cuddle/images/Cuddle-Banner.jpeg`
* **目的：** 独自のブランドとビジュアルを持つ、Linux 向けの高信頼原本保護型ファイル管理・作業環境・検索アプリ。
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
| [`README.ja.md`](file:///home/susie/git/wammed/Cuddle/README.ja.md) | [`README.md`](file:///home/susie/git/wammed/Cuddle/README.md) | プロジェクト概要・主要機能・クイックスタート案内 |
| [`BUILD.ja.md`](file:///home/susie/git/wammed/Cuddle/BUILD.ja.md) | [`BUILD.md`](file:///home/susie/git/wammed/Cuddle/BUILD.md) | ビルド環境構築・依存関係・テスト・本番ビルド手順書 |
| [`LICENSE`](file:///home/susie/git/wammed/Cuddle/LICENSE) | [`LICENSE`](file:///home/susie/git/wammed/Cuddle/LICENSE) | MIT License オープンソースライセンス条項 |
| [`FileWorkbench-Design-ver0.9.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.ja.md) | [`FileWorkbench-Design-ver0.9.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.en.md) | 統合基本設計書（残課題解決・バイリンガル仕様確定版） |
| [`FileWorkbench-Handover.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.ja.md) | [`FileWorkbench-Handover.en.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Handover.en.md) | 開発引き継ぎ書（本ドキュメント） |
| [`roadmap.ja.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.ja.md) | [`roadmap.en.md`](file:///home/susie/.gemini/antigravity-ide/brain/45a8eaa8-509d-4ed8-9e6c-255c45bac425/roadmap.en.md) | 詳細実装ロードマップ（全9フェーズ・28サブフェーズ） |
| [`NEXT_IMPLEMENTATION_TASKS.ja.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.ja.md) | [`NEXT_IMPLEMENTATION_TASKS.en.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.en.md) | 次期実装タスク計画書・設計仕様書（設計書ver 0.9照合18項目・Phase 10〜14） |
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
| **UI/UX スタイリング** | **Vanilla CSS + グラスモーフィズム** | 「シンプルだけど超リッチな見た目」を基本コンセプトとし、深色ベルベット（Dark Velvet Slate）、微細半透明ブラー（16px）、1px極細ヘアライン、ソフト発光パルスアニメーションを採用 |
| **タイポグラフィ & フォント** | **Pre-required（未同梱・システム前提）** | **Nerd Font は同梱しない（Zero Font Bundling）**。UI は `Inter` + Noto Sans CJK JP、コードは `JetBrains Mono`、グリフは `Symbols Nerd Font` 等をシステムフォントスタックから動的フォールバック参照。コアUIアイコンは `lucide-react` SVGベクターで100%描画保証 |

---

## 4. 現在の開発ステータスと進捗

* [x] **基本設計フェーズ：** v0.5〜v0.8 の統合および残課題解決、バイリンガル仕様を反映し `ver 0.9` として設計確定（日英両言語）。
* [x] **ロードマップ策定フェーズ：** 全9フェーズ・28サブフェーズに細分化、テストゲートとユーザー意思確認ゲート、i18n要件を定義してユーザー承認取得済み（日英両言語）。
* [x] **Phase 1.1（プロジェクト骨格 & 開発基盤構築）：完了**
  * Tauri 2 + React + TypeScript 骨格生成
  * Rust Edition 2021、クレート（tokio, thiserror, anyhow, serde, serde_json）整備
  * フロントエンド（i18next, @tanstack/react-virtual, lucide-react）整備
  * 完全バイリンガル辞書（en / ja）整備、動的言語切替、欠損キー検証テスト（Vitest 100% パス）
  * Rust ユニットテスト（Cargo test 100% パス）
* [x] **Phase 1.2（Landlock & セキュリティ機構の PoC 検証）：完了**
  * CachyOS カーネルの Landlock ABI 自動プローブ機構実装（実機検出: ABI V5）
  * `LandlockSandbox` ビルダー実装（`pre_exec` フックでのカーネルレベル隔離適用）
  * 隔離一時ディレクトリ（`TMPDIR`）環境変数注入機構
  * 子プロセス遮断テスト（原本ディレクトリへの書き込みが `EACCES` で確実に遮断されることを自動検証）
  * 外部ツール（ImageMagick `magick`）での原本保護実証テスト（パス）
  * Tauri コマンド `get_landlock_status` と UI セキュリティバッジの組み込み
* [x] **Phase 1.3（IPC 型定義 & 非 UTF-8 二重キー基盤）：完了**
  * `FileItem` 二重キー構造体実装（`item_id` UUID + `raw_base64` 原本バイト列 + `display_name` 安全UTF-8）
  * Linux `OsStrExt` を用いた完全バイト列保持・Base64 相互変換ユーティリティ
  * `ts-rs` による Rust 構造体から TypeScript 型定義（`src/types/`）の自動同期構築
  * 非 UTF-8 パス（不正バイト列 `\xFF\xFE`）の双方向変換ユニットテスト全件パス
  * Tauri コマンド `get_demo_items` および UI での非 UTF-8 ファイル耐性検証（文字化け置換文字 `` を安全表示、クラッシュゼロ）
* [x] **Phase 1.4（設定管理 TOML スキーマ・同期）：完了**
  * Linux XDG 仕様準拠 `$XDG_CONFIG_HOME/Cuddle/config.toml`（`dirs` + `toml` 0.8）
  * 安全ガードレール強制（Landlock 無効化試行の自動遮断・安全値強制復元、原本保護空き容量倍率 1.5x 下限強制）
  * 破損 TOML 検出時の `.bak` バックアップ自動退避および自己修復・安全既定値再生成
  * `ts-rs` による TypeScript 型自動同期（`AppConfig.ts`, `AppSection.ts`, `WorkspaceSection.ts`, `SecuritySection.ts`）
  * Tauri コマンド `get_app_config` / `save_app_config` 公開
  * UI 設定カードの実装および日英完全バイリンガル辞書整備
  * Cargo テスト 18 件 100% パス、Vitest 5 件 100% パス、Clippy 警告ゼロ
* [x] **Phase 2.1（高速ディレクトリスキャナ & inode 追跡）：完了**
  * `walkdir` (v2) による深さ制限（`max_depth`）・隠しファイル制御可能なディレクトリスキャナ実装（`src-tauri/src/scanner.rs`）
  * Linux `MetadataExt` を用いた物理識別情報（`inode: u64`, `dev: u64`）の `FileItem` 組み込みと `ts-rs` 型同期
  * 訪問済み `(dev, ino)` ハッシュセットによる循環シンボリックリンク無限ループ検知・自動安全スキップ機構
  * 非 UTF-8 不正バイト列（`\xFF\xFE`）ファイル名の安全走査・二重キー保持テスト全件パス
  * Tauri IPC コマンド `scan_dir` 公開
  * Cargo テスト 24 件 100% パス、Vitest 5 件 100% パス、Clippy 警告ゼロ
* [x] **Phase 2.2（plocate インデックス連携 & 高速ファイル名検索）：完了**
  * `plocate --null` による直接引数配列起動（シェル介在なし）でのミリ秒単位ファイル名検索実装（`src-tauri/src/search_filename.rs`）
  * `plocate` 不在・未インデックス・エラー時の `walkdir` 直接走査自動フォールバック機構
  * スコープディレクトリ制限（`scope_raw_base64`）および大文字小文字無視（`ignore_case`）・ベースネーム照合（`basename_only`）制御
  * `ts-rs` による TypeScript 型自動同期（`SearchFilenameOptions.ts`, `SearchResultDto.ts`）
  * Tauri IPC コマンド `search_files_by_name` 公開
  * Cargo テスト 30 件 100% パス、Vitest 5 件 100% パス、Clippy 警告ゼロ
* [x] **Phase 2.3（Tantivy + lindera-tantivy 日本語全文検索エンジン）：完了**
  * Tantivy 0.26 および Lindera 5（`embed-ipadic` 形態素解析トークナイザー内蔵）による日本語全文検索エンジン実装（`src-tauri/src/search_fulltext.rs`）
  * Linux XDG 仕様準拠 `$XDG_DATA_HOME/Cuddle/index/` 永続インデックスおよびテスト用インメモリ（`new_in_ram`）エンジン
  * `item_id` に基づく安全な差分ドキュメント更新（重複排除 `delete_term` + `add_document`）および明示的削除機能
  * `SnippetGenerator` による検索マッチ箇所のハイライトプレビュー要約スニペット自動抽出
  * `ts-rs` による TypeScript 型自動同期（`FulltextHitDto.ts`, `FulltextSearchOptions.ts`）
  * Tauri IPC コマンド `search_fulltext` 公開と `invoke_handler` 登録
  * Cargo テスト 35 件 100% パス、Vitest 5 件 100% パス、Clippy 警告ゼロ
* [x] **Phase 2.4（テキスト抽出分離ワーカー：プレーンテキスト/MD直接抽出 + メモリ制限付き分離プロセス）：完了**
  * 2段階テキスト抽出アーキテクチャ実装（`src-tauri/src/extractor.rs`）
  * Tier 1 高速直接抽出（テキスト・コード系）＆ Mozilla `chardetng` / `encoding_rs` による日本語文字コード（UTF-8, Shift_JIS, EUC-JP）自動判定
  * 10 MiB サイズ上限ガード＆最大文字数切り詰め（`truncated`）安全設計
  * Tier 2 分離ワーカー（PDF）：Landlock LSM による原本読み取り専用保護、RLIMIT_AS（256 MiB）メモリ上限、30秒タイムアウト付き安全実行
  * `pdftotext` 不在時の安全スキップ・警告通知ハンドリング
  * テキスト抽出から Tantivy + Lindera 日本語全文検索インデックスへの自動パイプライン連携（`index_file_with_text`）
  * `ts-rs` による TypeScript 型自動同期（`ExtractedTextDto.ts`, `ExtractTextOptions.ts`）
  * Tauri IPC コマンド `extract_file_text`, `index_file_with_text` 公開
  * 日英完全バイリンガル辞書（`extractor.*`）整備および UI 抽出カード追加
  * Cargo テスト 45 件 100% パス、Vitest 5 件 100% パス、Clippy 警告ゼロ
* [x] **Phase 3.1（6領域メインレイアウト UI 統合 & @tanstack/react-virtual 仮想スクロール結合）：完了**
  * `react-resizable-panels` による 6 領域スプリットペイン統合（`src/App.tsx`）
  * 左列（検索結果＋ディレクトリツリー）、中央列（ファイルブラウザー＋作業ワークスペース）、右列（詳細インスペクターサイドバー）、下部列（プレビュー／履歴／セキュリティ／ログ）
  * `@tanstack/react-virtual` による Standard 密度（行高 42px: アイコン、名前、サイズ、更新日時、inode、原本保護バッジ）の 60fps 仮想スクロール（`src/components/VirtualFileList.tsx`）
  * パネルコンポーネント群の実装（`Header`, `SearchPanel`, `FolderTreePanel`, `FileBrowserPanel`, `WorkspacePanel`, `DetailsSidebarPanel`, `BottomTabPanel`）
  * 右サイドバー・下部パネルの開閉トグル＆レイアウト初期化機能
  * 深色ベルベット（Dark Velvet Slate `#070b12`）グラスモーフィズムデザイン
  * 完全バイリンガル辞書キー（`panels.*`, `browser.*`, `tree.*`, `details.*`）追加＆動的切替
  * Cargo テスト 45 件 100% パス、Vitest 6 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 3.2（安全操作層 OperationPolicy エンジン & 原本保護・競合検出）：完了**
  * 安全操作ポリシーエンジン実装（`src-tauri/src/policy.rs`）
  * `/proc/mounts` 解析による外部ストレージ・マウント境界・`$HOME` 外の禁止領域動的検出機構（`is_external_or_mount_boundary`, `is_in_home_directory`）
  * 原本反映（Reflection）に対する絶対的原本保護ガードレール（`validate_reflection` による `ERR_FORBIDDEN_ZONE` 強制遮断）
  * 原本スナップショット記録（`OriginalSnapshotDto`：inode, dev, size, mtime, SHA-256）
  * TOCTOU 外部競合検出エンジン（`verify_conflict`：inode 置換、ファイルサイズ変動、mtime 変更、SHA-256 ハッシュ不整合の厳格検知）
  * `libc::statvfs` を用いた作業領域ディスク空き容量ガードレール（原本サイズの 1.5 倍下限チェック：`check_disk_space`）
  * `ts-rs` による TypeScript 型自動同期（`OriginalSnapshotDto.ts`, `PolicyCheckResultDto.ts`）
  * Tauri IPC コマンド `check_operation_policy`, `create_original_snapshot`, `verify_original_conflict` 公開
  * 日英完全バイリンガル辞書（`policy.*`, `errors.ERR_*`）整備および詳細サイドバーでのリアルタイム安全性評価・スナップショット検証 UI 連携
  * Cargo テスト 54 件 100% パス、Vitest 8 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 4.1（作業コピー分離 & 同一ボリューム優先配置）：完了**
  * 作業コピー隔離エンジン実装（`src-tauri/src/workspace.rs`）
  * 原本と同一ボリューム内の `.cuddle-temp/` 優先配置および Linux `ioctl(FICLONE)` による Reflink (CoW) 瞬時クローン実装
  * Reflink 非対応ファイルシステムまたは書き込み不可時の `$XDG_CACHE_HOME/Cuddle/staging/` への自動フォールバック
  * 事前空き容量ガードレール（原本サイズの 1.5 倍以上空き容量必須チェック）
  * 二重キー整合性と作業コピー状態追跡（`StagedItemDto`：原本情報、ステージング先パス、クローン方式 `Reflink` | `StandardCopy`、状態 `Staged` | `Modified` | `Reflected`、原本スナップショット）
  * `ts-rs` による TypeScript 型自動同期（`StagedItemDto.ts`, `CloneMethod.ts`, `StageStatus.ts`）
  * Tauri IPC コマンド `stage_file`, `unstage_file`, `list_staged_items`, `clear_staged_items` 公開
  * `WorkspacePanel.tsx` および `VirtualFileList.tsx` での Reflink バッジ表示、作業解除、一括解除 UI 統合
  * 日英完全バイリンガル辞書（`workspace.*`, `errors.ERR_STAGE_*`）整備
  * Cargo テスト 61 件 100% パス、Vitest 10 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 4.2（作業領域内高速スワップ・差分比較 & 原本保護下プレビュー）：完了**
  * `similar` (v2.6) クレートを用いた高速テキスト差分比較エンジン実装（`src-tauri/src/diff.rs`）
  * 先頭バイト解析によるバイナリ自動検知および安全なバイナリ差分ハンドリング
  * 構造化差分 DTO（`FileDiffResultDto`, `DiffLineDto`, `DiffChangeType`）の行番号（旧/新）追跡
  * ワンクリック原本状態復元エンジン（`reset_staged_file`：作業コピー破棄＋スナップショット時点の原本からの再クローン・状態リセット）
  * `ts-rs` による TypeScript 型自動同期（`DiffChangeType.ts`, `DiffLineDto.ts`, `FileDiffResultDto.ts`）
  * Tauri IPC コマンド `get_file_diff`, `reset_staged_file` 公開
  * `BottomTabPanel.tsx` プレビュータブでの原本 vs 作業コピー差分（追加: 緑, 削除: 赤, 変更なし: 通常色）リアルタイム表示
  * プレビューヘッダーでの「原本状態へ復元 (リセット)」ボタン連携
  * 日英完全バイリンガル辞書（`diff.*`, `errors.ERR_DIFF_FAILED`）整備
  * Cargo テスト 69 件 100% パス、Vitest 12 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 4.3（安全な原本反映トランザクション・競合遮断 & TOCTOU レース防御）：完了**
  * 安全な原本反映（Reflection）トランザクションエンジン実装（`src-tauri/src/workspace.rs`）
  * 反映前の厳格な多重ガードレール：
    * 禁止領域動的検証（`validate_reflection` による `$HOME` 外、マウント境界、外部ストレージへの書き込み強制遮断）
    * TOCTOU 競合検出（`verify_conflict` による SHA-256 ハッシュ、mtime、inode、ファイルサイズ不整合の自動遮断）
    * 反映先ディスク空き容量チェック
  * 同一親ディレクトリ内一時退避ファイル（`.cuddle_reflect_<uuid>.tmp`）と Linux POSIX `fs::rename` による同一ファイルシステム内完全アトミック置換
  * 置換成功後の原本スナップショット自動再取得（SHA-256更新）およびステージング状態 `StageStatus::Reflected` 移行
  * Tauri IPC コマンド `reflect_staged_file` 公開と `invoke_handler` 登録
  * `BottomTabPanel.tsx` 差分ヘッダーへの「原本へ変更を反映」ボタン追加および安全確認モーダルダイアログ（原本・作業コピーパス、変更行数、警告表示）統合
  * `App.tsx` での反映 IPC 実行とブラウザー一覧の自動再スキャン・サイズ同期
  * 日英完全バイリンガル辞書（`reflection.*`）整備
  * Cargo テスト 71 件 100% パス、Vitest 13 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 5.1（正規化・一括リネームルールエンジン & ドライラン検証）：完了**
  * `batch.rs` による一括リネーム・正規化エンジン実装（正規表現置換、Unicode NFC/NFD/NFKC/NFKD、全角半角変換、連番付与、日時スタンプ、接頭辞・接尾辞）
  * リアルタイム衝突検査（バッチ内重複衝突 `ConflictDuplicate`、ディスク上既存衝突 `ConflictExistsOnDisk`、不正文字 `InvalidFilename`）
  * 仮想マッピング一覧（`BatchPlanDto`, `PlannedOperationDto`）の即時生成と Dry-run 検証
  * `BulkRenameModal.tsx` によるレスポンシブなルール設定UI・プレビュー差分テーブル・実行保護の実装
  * `ts-rs` による TypeScript 型自動同期（`BatchPlanDto.ts`, `PlannedOperationDto.ts`, `RenameRule.ts`, `BulkRenameOptions.ts` 等）
  * 日英完全バイリンガル辞書（`bulk_rename.*`）整備
* [x] **Phase 5.2（段階リネーム Two-Phase Rename & 全体アトミックロールバック）：完了**
  * 一時 UUID 名への退避（Phase 1）→ 目的名への確定（Phase 2）によるスワップ・連番重複の安全な解決
  * 途中エラー発生時の逆順ロールバック（RollbackAll）による作業領域の整合性保護
  * `WorkspaceManager::update_staged_path` によるステージング領域・ファイルブラウザとのリアルタイム同期
* [x] **Phase 5.3（ロールバックジャーナル & 一括アンドゥ）：完了**
  * `BatchJournalManager` によるスレッドセーフなリネーム履歴スタックの管理
  * 逆変換計画の自動生成と Two-Phase Rename によるワンクリック安全復元（`undo_last_bulk_rename`）
  * ステージングファイル UUID プレフィックスの内部保護とユーザー表示名（`display_name`）の完全復元保証
  * 作業領域ヘッダーへのアンドゥボタン（`RotateCcw`）統合、フローティングトースト通知バナーによる即時フィードバック
  * Cargo テスト 96 件 100% パス、Vitest 20 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 6.1（操作履歴・永続ジャーナル & UNDO 分離モデル）：完了**
  * `rusqlite` (bundled) による SQLite WAL モード & Normal 同期での操作履歴マネージャー実装（`src-tauri/src/history.rs`）
  * ステージング、アンステージ、一括リネーム、アンドゥ、原本復元、原本反映、全クリア等の各操作を自動記録
  * 履歴上限 50 件の自動トリミング機能（最新 50 件保持）
  * Tauri IPC コマンド `get_operation_history`, `clear_operation_history` 公開
  * `BottomTabPanel.tsx`「操作履歴」タブの刷新：タイムスタンプ、操作種別バッジ（色分け）、対象ファイル、原本パス、ステータス、詳細情報を含むタイムライン表示
  * 履歴消去ボタンと即時 UI 同期、グローバルステータスバーでの記録件数表示
  * 日英完全バイリンガル辞書（`history.*`）整備
  * Cargo テスト 100 件 100% パス、Vitest 22 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 6.2（セッション復元エンジン & TOCTOU 競合検知）：完了**
  * SQLite WAL `session_items` テーブルによるステージング状態の永続マニフェスト化（ステージング・アンステージ・リネーム・アンドゥ・リセット・反映・クリア時に自動同期）
  * アプリ起動時の自動セッション復元エンジン（`restore_workspace_session`）実装
  * 復元時のディスク実在確認および原本ファイルに対する厳格な TOCTOU 競合検査（SHA-256・mtime・サイズ）
  * 外部変更検知時の安全な `StageStatus::Conflict` 遷移と警告バッジ（`conflict-pill-sm`）表示
  * `ts-rs` による TypeScript 型自動同期（`StageStatus.ts` に `"Conflict"` 追加）
  * 日英完全バイリンガル辞書（`workspace.status_conflict`, `workspace.restored_session`, `workspace.conflict_alert`）整備
  * Cargo テスト 102 件 100% パス、Vitest 23 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 6.3（終了時対話バックアップ & tar.gz アーカイブ）：完了**
  * 未反映ファイル自動検知（`has_unreflected_staged_items`）およびデフォルトバックアップ保存先パス生成（`get_default_backup_path`：`$XDG_DATA_HOME/Cuddle/backups/backup_YYYYMMDD_HHMMSS.tar.gz`）
  * `flate2` + `tar` による作業領域内全ステージングファイルと `manifest.json`（原本パス、ステージング名、クローン方式、タイムスタンプ）の tar.gz 高速アーカイブ生成エンジン（`create_workspace_backup`）
  * ユーザー意思決定ゲートに基づく柔軟な保存先指定（デフォルトパスのワンクリック復元＋任意カスタムパス手動指定・親ディレクトリ自動再帰作成）
  * バックアップ作成完了時の SQLite WAL 操作履歴への自動記録（`OperationType::Backup`）
  * `ExitBackupModal.tsx` 実装：「アーカイブ保存して終了」「作業コピーを破棄して終了」「作業を継続する（キャンセル）」の 3 通り安全選択肢、エラーバナー、パス入力欄
  * `WorkspacePanel.tsx` ヘッダーへの「バックアップ / 終了」ボタン（`Archive` アイコン）統合
  * `ts-rs` による TypeScript 型自動同期（`BackupResultDto.ts`）
  * 日英完全バイリンガル辞書（`backup.*`）整備
  * Cargo テスト 105 件 100% パス、Vitest 31 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功
* [x] **Phase 7.1（外部コマンド安全実行エンジン & ツール検出 & 純Rust高速変換）：完了**
  * `src-tauri/src/pipeline.rs` による安全隔離実行・外部ツール自動検出・純Rust変換エンジン実装
  * **Landlock LSM 隔離:** 原本パスへのアクセスを遮断し、`workspace-temp/` 領域のみ読み書き許可で子プロセスを実行
  * **ガードレール保護:** RLIMIT_AS (512 MiB) 仮想メモリ上限、30 秒 SIGKILL タイムアウト保護、10 MiB 出力切り詰めガード、シェル非経由（`sh -c` 排除）の引数配列直接プロセス起動
  * **純 Rust 変換エンジン:** 外部ツール未インストール環境でも即時稼働する軽量高速変換（ログ機密マスキング、CSV クレンジング、JSON Minify、TSV/CSV 相互変換、SQL ダンプ軽量化、SRT 字幕タイムシフト、SHA-256 チェックサム生成）
  * **Tauri IPC コマンド:** `probe_external_tools`, `execute_sandboxed_command`, `run_builtin_text_transform` を公開
  * **UI 統合:** `BottomTabPanel.tsx`「セキュリティ & 設定」タブ内に「外部ツール連携環境 (Phase 7.1)」カードを配備（ImageMagick, FFmpeg, qpdf, pdfimages, exiftool, pandoc の可用性・バージョン・パス表示、再スキャンボタン）
  * **ダークテーマ統一:** 白背景完全禁止、Dark Velvet Slate パレット準拠
  * **型自動同期:** `ts-rs` による TypeScript 型自動生成（`ExternalToolStatusDto.ts`, `CommandExecutionOptionsDto.ts`, `CommandExecutionResultDto.ts`, `BuiltinTransformResultDto.ts`）
* [x] **Phase 7.2（30種パイプラインプリセット定義 & DAG 実行エンジン & UI 統合）：完了**
  * **30種プリセット定義:** 画像 1〜8、音声動画 9〜13、テキスト 14〜20、ドキュメント 21〜24、ファイル整理 25〜27、開発配布 28〜30 の全プリセットを `PipelineRegistry` に実装。
  * **ハイブリッド実行エンジン:** 純Rust高速変換と Landlock LSM 隔離外部ツール実行（ImageMagick, FFmpeg, qpdf 等）のハイブリッド構成。
  * **原本完全保護:** 作業領域（`workspace-temp/`）内コピーに対してのみ変換を適用し、ステージングパスを安全に更新。原本は一切変更されない。
  * **操作履歴連携:** 実行完了時に SQLite WAL 操作履歴へ `OperationType::Pipeline` として自動記録。
  * **UI 統合（`PipelineModal.tsx`）:** 30種プリセットブラウザ、カテゴリピル（すべて/画像/メディア/テキスト/ドキュメント/ファイル/開発）、検索、パラメータ調整スライダー、外部ツール要否判定、実行結果サマリー、多重クリック防止。
  * **UI ダークモード徹底:** 「原本スナップショット記録済み」「テキスト抽出 & インデックス登録」ボタン等の白っぽさを完全解消し、深みのある Dark Velvet Slate（`rgba(15, 23, 42, 0.9)`、文字 `#e2e8f0`、枠線 `rgba(255, 255, 255, 0.14)`）へ統一。テキストボックス・モーダル・ポップアップの白背景完全禁止を徹底。
  * **完全バイリンガル同期:** 日英両言語の辞書（`pipeline.*`）を完全同期。
  * **型自動同期:** `ts-rs` による TypeScript 型自動生成（`PipelineCategory.ts`, `PipelinePresetDto.ts`, `PipelineParameterDef.ts`, `ExecutePipelineRequestDto.ts`, `PipelineExecutionResultDto.ts`, `PipelineExecutionItemResultDto.ts`）。
* [x] **Phase 7.3（リアルタイム進捗通知 & キャンセル制御 & UI 統合）：完了**
  * **非同期ストリーミング通知:** 各ファイルの処理開始・完了・失敗状況を Tauri イベント（`"pipeline-progress"`）でリアルタイム emit。
  * **ジョブ単位のキャンセル管理:** `PipelineCancellationManager` によるアトミックフラグ管理（`cancel_pipeline` コマンド）。
  * **子プロセスの即時 SIGKILL:** 外部ツール（ImageMagick、FFmpeg 等）実行中であっても、ユーザーのキャンセル要求を検知して即時に子プロセスを終了し安全に中断。
  * **UI 統合（プログレスバー & 中断制御）:**
    * 実行中にパーセンテージ、進捗件数（例: `1 / 5`）、現在処理中のファイル名をプログレスバーと共にリアルタイム表示。
    * 「パイプラインを実行」ボタンが実行中に赤系ダークサーフェスの「処理を中断（キャンセル）」ボタン（`StopCircle` アイコン）に切り替わり、多重クリックを防止。
    * ユーザーキャンセル時にアンバー色警告バナー（`AlertTriangle`）で中断完了状況を明確にフィードバック。
  * **UI ダークモード徹底:** プログレスバー（Dark Velvet Slate ベース）、キャンセルボタン、警告バナーすべて白背景を完全排除。
  * **完全バイリンガル同期:** 日英辞書に進捗・キャンセル関連キーを追加・同期。
  * **テスト・ビルド検証:** Cargo テスト 130 件 100% パス、Vitest 38 件 100% パス、Clippy 警告ゼロ、Vite ビルド成功。

---

## 5. フェーズ運用ルール & 進行プロトコル（厳守事項）

今後の実装進行時は、必ず以下のプロトコルを順守すること。

1. **フェーズ開始前意思決定ゲート：**  
   各サブフェーズ（Phase X.Y）の実装に入る前に、該当フェーズで確定すべき技術選定・仕様・選択肢をユーザーに提示し、合意・意思決定を得てから実装を開始する（※無断着手禁止）。
2. **完全バイリンガル実装の徹底：**  
   * 実装するすべての UI、メッセージ、エラーテキストは `locales/en/*.json` および `locales/ja/*.json` の両方に定義する。ハードコードされた単一言語テキストを一切残さない。
   * 作成・更新するすべてのドキュメント（README、設計書、タスクシート等）は同内容の日英両言語で作成する。
3. **カラーテーマ & パイプラインプリセットの通知合意ルール：**
   * **カラーテーマ:** UI の最終調整時（Phase 8〜9）の実装フェーズに入る時点で事前通知し、ユーザーと合意形成を行う。
   * **パイプラインプリセット:** 推奨タイミング（Phase 7: パイプラインランナー）の実装フェーズに入る時点で事前通知し、ユーザーと合意形成を行う。
4. **メジャーフェーズ移行時の手動テスト実施：**  
   メジャーフェーズ（Phase 1 → 2、Phase 2 → 3、Phase 3 → 4、Phase 4 → 5、Phase 5 → 6、Phase 6 → 7 等）が切り替わる境界において、必ず実機 GUI / CLI 上での手動テストチェックリストをユーザーに提示し、動作検証ステップを実施・確認した上で次メジャーフェーズへ進む。

---

## 6. 次期セッション着手手順（Phase 7 のタスク）

メジャーフェーズ 6（バックアップ・履歴・UNDO・セッション復元）の自動テストがすべて完了したため、**手動テストチェックリストによる実機検証確認**および**フェーズ開始前意思決定ゲートの合意**を経てメジャーフェーズ 7（外部ツール連携 & パイプラインランナー）へ移行する。

### 7.1 Phase 7.1 の目的と設計方針（外部コマンド安全実行エンジン）

* **プロセス分離 & Landlock LSM / cgroups 安全隔離実行：**
  * 外部ツール（ImageMagick、FFmpeg、pandoc 等）の実行を `workspace-temp/` 領域のみ読み書き許可で隔離実行。
  * メモリ・CPU・実行時間タイムアウト（30秒等）の上限保護。
  * シェル文字列展開（`sh -c`）を完全排除した引数配列実行。

### 7.2 Phase 7.2 の目的と設計方針（30種パイプラインプリセット & DAG ワークフロー）

* **合意済み 30 種パイプラインプリセット体系：**
  * **画像・ビジュアル処理系（1〜8）：**
    1. Web配信用軽量化（Exif削除 $\rightarrow$ 1920px縮小 $\rightarrow$ WebP品質80 $\rightarrow$ `_web`付与）
    2. SNS・OGPカード生成（1200x630クロップ $\rightarrow$ 余白パディング $\rightarrow$ JPEG品質85）
    3. アプリアイコン・ファビコンパック（正方形クロップ $\rightarrow$ 16/32/180/512pxマルチ出力 $\rightarrow$ PNG）
    4. 透かし入りプレビュー画像出力（1200px縮小 $\rightarrow$ 右下30%透かし合成 $\rightarrow$ JPEG）
    5. UIアセット用Retina縮小セット（@2x入力 $\rightarrow$ 50%縮小@1x生成 $\rightarrow$ `name@2x.png`/`name.png`）
    6. ドット絵・スプライト拡大（ニアレストネイバー400%拡大 $\rightarrow$ 透過PNG保持）
    7. 印刷入稿用CMYK/高解像度化（300DPI $\rightarrow$ sRGB to CMYK $\rightarrow$ 非可逆なしTIFF）
    8. 写真の白黒アーティスティック加工（グレースケール $\rightarrow$ S字トーンカーブ $\rightarrow$ ノイズ微量付加 $\rightarrow$ JPEG）
  * **音声・動画・リッチメディア系（9〜13）：**
    9. 動画からWeb用GIFアニメ生成（区間トリム $\rightarrow$ 幅480px $\rightarrow$ パレット最適化 $\rightarrow$ GIF）
    10. ポッドキャスト配信向けマスタリング（ラウドネス-16 LUFS $\rightarrow$ 無音トリム $\rightarrow$ 128kbps MP3 $\rightarrow$ ID3タグ）
    11. 音声抽出・文字起こし前処理（映像破棄 $\rightarrow$ モノラル $\rightarrow$ 16kHz サンプリング $\rightarrow$ WAV）
    12. スマホ動画のSNS用縦型成型（9:16センタリングクロップ $\rightarrow$ H.264/AAC $\rightarrow$ FastStart）
    13. 高解像度動画のDiscord/Slack共有用圧縮（720pダウン $\rightarrow$ 2パス目標8MB以内 $\rightarrow$ MP4）
  * **テキスト・ログ・データ整形系（14〜20 / 内部Rust高速エンジン対応）：**
    14. 公開用ログの機密情報マスキング（IP/メール/APIキー置換 $\rightarrow$ 日付プレフィックス）
    15. 基幹システムCSVのUTF-8クレンジング（Shift_JIS to UTF-8 $\rightarrow$ CRLF to LF $\rightarrow$ トリム）
    16. JSONデータのフォーマット・圧縮（構文検証 $\rightarrow$ Minify空白改行除去 $\rightarrow$ `.min.json`）
    17. TSV・CSV相互変換＆ヘッダー小文字化（タブ/カンマ変換 $\rightarrow$ スネークケース $\rightarrow$ エスケープ）
    18. Markdown技術文書の整形・リンター（全角半角正規化 $\rightarrow$ 表組み空行フォーマット）
    19. SQLダンプファイルの軽量化（コメント空行削除 $\rightarrow$ INSERTバルク結合 $\rightarrow$ gzip圧縮）
    20. 字幕ファイル（SRT）タイムコードシフト・成型（タイムコード一括オフセット $\rightarrow$ タグ除去 $\rightarrow$ UTF-8）
  * **ドキュメント・PDF処理系（21〜24）：**
    21. スキャン書類の可読性向上＆白黒二値化（Deskew傾き補正 $\rightarrow$ グレースケール $\rightarrow$ 適応的二値化 $\rightarrow$ PDF）
    22. 複数PDFの一括統合＆しおり生成（A4統一 $\rightarrow$ ファイル名順結合 $\rightarrow$ 目次アウトライン付与）
    23. 配布用PDFのセキュリティ適用（メタデータ消去 $\rightarrow$ 印刷・コピー制限パスワード $\rightarrow$ 圧縮保存）
    24. PDFからの画像一括抽出（埋め込みラスタ抽出 $\rightarrow$ 重複ハッシュチェック $\rightarrow$ 連番出力）
  * **ファイル管理・整理・リネーム系（25〜27）：**
    25. 写真・動画の撮影日時ディレクトリ自動仕分け（Exif日時読み取り $\rightarrow$ `YYYYMMDD_HHMMSS` $\rightarrow$ `YYYY/MM/`仕分け）
    26. ダウンロード散乱ファイルの一括正規化（URLエンコード記号除去 $\rightarrow$ スペースをハイフン化 $\rightarrow$ 小文字化）
    27. 連番アセットのゼロパディング揃え（末尾数値検出 $\rightarrow$ 4桁ゼロ埋め `img_0001.png` $\rightarrow$ 衝突回避）
  * **開発・配布・セキュリティ系（28〜30）：**
    28. ソースコード配布用クリーンパッケージ（`.git`/`node_modules`除外 $\rightarrow$ 改行LF統一 $\rightarrow$ ZIP）
    29. 改ざん検知用チェックサムファイル同梱アーカイブ（SHA-256ハッシュ算出 $\rightarrow$ `checksums.txt` $\rightarrow$ tar.gz）
    30. 機密ファイルの暗号化バックアップ（tar化 $\rightarrow$ AES-256暗号化 $\rightarrow$ `.enc`付与）
* **ツール可用性プローブ（Tool Availability Probe）：**
  * 各プリセットの必要ツール（`magick`, `ffmpeg`, `qpdf`, `pdfimages`, `exiftool` 等）の `$PATH` 実在を検知し、未インストール時は分かりやすい案内を表示。純Rust完結プリセット（14〜20, 26〜29）は外部ツール不要で即時利用可能。

### 7.3 Phase 7.3（リアルタイム進捗 UI & キャンセル制御）：完了

* **非同期ストリーミング通知と UI 統合：**
  * 各ファイルの処理状況（待機・実行中・成功・失敗）をリアルタイムに進捗プログレスバーで表示（Tauri イベント `pipeline-progress`）。
  * `PipelineCancellationManager` による子プロセスの即時 SIGKILL 中断と RollbackAll 安全終了。
  * `PipelineModal.tsx` にプログレスバー、処理中ファイル名、深紅キャンセルボタン、中断警告バナーを完全統合。

---

## 8. Phase 8（UI/UX 最終調整、仮想スクロール、レイアウト最適化 & カラーテーマ）：完了

### 8.1 ユーザー合意事項（意思決定ゲート）

1. **カラーテーマ選定：** `Midnight Teal / Cyber Dark`（深海ダーク `#060c11`, `#09141b` をベースに、サイバーティール `#14b8a6`、ネオンミント `#2dd4bf` のアクセントを取り入れたサイバー調ダークパレット）。
2. **ボタン隠れ問題解決方針：** `下部固定アクションバー方式`（全モーダル・ダイアログでフッターボタンを画面下部に固定配置し、入力・設定・リスト領域のみを独立スクロールさせて、ウィンドウが小さくてもボタンが絶対に隠れない構造に統一）。

### 8.2 実装内容

* **下部固定アクションバー（Fixed Bottom Action Bar）：**
  * `.reflect-confirm-dialog`, `.bulk-rename-modal`, `.exit-backup-dialog`, `.pipeline-modal-dialog` の全モーダルにおいて、共通アクションバー（`.dialog-footer`, `.dialog-actions-bar`）に `flex-shrink: 0; margin-top: auto; position: relative; z-index: 10;` を設定。
  * コンテンツボディ部（`.dialog-body`, `.bulk-rename-body`, `.pipeline-config-scroll`）に `flex: 1; min-height: 0; overflow-y: auto;` を徹底し、エラーバナーやプレビューテーブルが長くなってもボタンバーが画面内に常に固定表示される構造を確立。
* **Midnight Teal / Cyber Dark カラーテーマの統一：**
  * `--bg-canvas` (`#060c11`), `--bg-surface-base` (`#09141b`), `--bg-surface-input` (`#050a0e`), `--accent-brand` (`#14b8a6`), `--accent-brand-bright` (`#2dd4bf`)。
  * 幅 6px のティールカスタムスクロールバー（`::-webkit-scrollbar`）を全コンポーネントにグローバル適用。
  * 「原本スナップショット記録済み」「テキスト抽出 ＆ インデックス登録」等の白浮きボタンを重厚なサイバーダーク背景＋高コントラストミント文字へ刷新。
  * 全入力要素・ポップアップ・モーダルにおける白背景を完全根絶。
* **仮想スクロール（`@tanstack/react-virtual`）：**
  * `FileBrowserPanel` および `WorkspacePanel` にて `VirtualFileList` を稼働。最大 10,000 件のファイル一覧を 60fps で滑らかに描画し、メモリ消費を最小化。
* **キーボードショートカット & アクセシビリティ（Phase 8.4）：**
  * `Ctrl+F` / `Cmd+F`: グローバル検索入力フィールド（`#global-search-input`）へ即座にフォーカス & 全選択。
  * `Ctrl+R` / `Cmd+R`: 一括リネームモーダルを開く（ワークスペースにファイルがある場合）。
  * `Ctrl+P` / `Cmd+P`: パイプラインモーダルを開く（ワークスペースにファイルがある場合）。
  * `Ctrl+Z` / `Cmd+Z`: 直前のリネーム操作を取り消す（Undo 履歴がある場合。テキストエリア内ではネイティブ Undo を優先）。
  * `Escape`: 開いているモーダルを安全に閉じる。
* **FHDウィンドウサイズ（1920x1080）:**
  * `src-tauri/tauri.conf.json` の既定ウィンドウサイズを `width: 1920, height: 1080`（最小 1024x700）に固定。
* **ヘッダーバーの2行構成化（表示崩れ防止）：**
  * `Header.tsx` を2行構成に刷新。1行目にブランドロゴ・タイトル、モード切替タブ（スタート ↔ ワークベンチ）、Landlockセキュリティバッジ、設定ボタン（⚙️）、言語切替を配置。2行目に現在パス表示とパネル切替ボタンを配置。日本語ラベルでも絶対に圧縮・改行崩れしないレイアウトを確立。
* **設定メニューモーダル（`SettingsModal.tsx`）の実装:**
  * ヘッダーのギアアイコン（⚙️）から開く設定モーダルを新設。30色テーマのライブ切り替え、言語切替、ワークスペースパス、1.5x 空き容量ガードレール、Landlock 隔離状態、ショートカット一覧、`config.toml` パスを完全網羅。
* **30種類のカラーテーマプリセット（`themePresets.ts`）：**
  * `Cyber`（8種: Midnight Teal, Matrix, Quantum Abyss, Circuit, Void, Biohazard, Glitch, Subzero）
  * `Neon`（8種: Synthwave, Cyberpunk 2077, Hotline Miami, Laser Pink, Acid Lime, Electric Blue, Ultra Violet, Solar Flare）
  * `Modern`（8種: Tokyo Night, Nord Frost, Dracula, Catppuccin, One Dark Pro, Gruvbox Dark, Monokai Pro, Rosé Pine）
  * `Classic`（6種: Dark Charcoal, Midnight Navy, Obsidian Black, Industrial Slate, Dark Espresso, Emerald Night）
  * すべてダークモード前提（白背景禁止）で設計。CSS変数による即時切り替えと `localStorage` 永続化を実装。
* **設定ファイルテンプレート（`config.template.toml`）の作成:**
  * プロジェクトルートに詳細なコメント付きテンプレートを配置。30種テーマキー一覧やガードレール仕様を完全記載。
* **スタートページ（`StartPage.tsx`）の設計・実装:**
  * ホーム/ワークベンチ切替式の洗練されたウェルカムダッシュボード。3大機能カード（ブラウザ走査、30種パイプライン、一括リネーム）、クイックロケーション、Landlock & 空き容量のリアルタイムシステム監視を統合。
* **ファイルブラウザ & ワークスペースのパネルヘッダー2行ツールバー化（表示崩れの完全根絶）：**
  * **問題の根本原因：** ファイルブラウザの「タイトル＋長いパスバー＋投入ボタン群」や、ワークスペースの「タイトル＋保護バッジ＋5つの操作ボタン（パイプライン、リネーム、元に戻す、バックアップ、全クリア）」が1行（38px固定）に横並びしていたため、ペイン幅が約880px（全体でWQHD 2560pxフルスクリーン相当）に達しないと要素が折り返され重なりやはみ出しを起こしていた。
  * **ファイルブラウザ（`FileBrowserPanel.tsx`）：**
    * 1行目: `FolderOpen` アイコン＋「ファイルブラウザ」タイトル、右端に「選択を作業領域へ」「すべて作業領域へ」「更新」ボタン。
    * 2行目: 「↑ 親へ」ボタン＋全幅パスバー（`browser-path-toolbar-row`）。パスが長くてもボタンを圧迫せず、クリックでインライン編集可能。
  * **ワークスペース（`WorkspacePanel.tsx`）：**
    * 1行目: `Briefcase` アイコン＋「作業ワークスペース」＋`workspace-temp/` 保護バッジ、右端に「すべて解除（危険操作）」ボタン。
    * 2行目: 操作専用ツールバー行（`workspace-action-toolbar-row`）。「パイプライン」「一括リネーム」「直前のリネームを元に戻す」「バックアップ / 終了」を整然と配置。
  * **CSSの堅牢化（`App.css`）：**
    * 全ボタンに `white-space: nowrap; flex-shrink: 0;` を徹底付与。
    * 各行の幅が狭い場合でも `flex-wrap: wrap` により自然に折り返され、ペイン幅が400px程度の狭いウィンドウ（FHDやコンパクトサイズ）でも絶対に崩れないレイアウトを確立。
* **Git 情報タブ（Git Status Dashboard）（Phase 8.5）：**
  * `src-tauri/src/git.rs` による高精度 Git ステータスバックエンド実装。
  * `git rev-parse --show-toplevel`, `git status --porcelain=v2 --branch`, `git log -1` を直接引数配列起動（シェル介在なし）で高速実行。
  * 生テキスト出力ではなく、構造化 DTO（`GitStatusDto`, `GitFileItemDto`）として取得。
  * ブランチバッジ、Upstream 同期情報（Ahead / Behind）、クリーン判定、最新コミットカード（コミットハッシュ、メッセージ、作者、日時）、および 4 大カテゴリ（ステージ済み変更 `staged`、未ステージの変更 `unstaged`、未追跡ファイル `untracked`、マージ競合 `conflicts`）に項目分割して色分けバッジ（M, A, D, R, U, ?）でリッチ表示。
  * `ts-rs` による TypeScript 型同期（`GitStatusDto.ts`, `GitFileItemDto.ts`）と Cargo ユニットテスト 6 件パス。
* **タブ表示名の完全整合（Phase 8.5）：**
  * 「セキュリティ & 設定」と混同されていた下部タブを、設計書に準拠した正式名称「セキュリティ & ツール診断」（英語: "Security & Tools Diagnostic"）へ完全に整合（日英辞書キー `panels.tab_security` 更新）。
* **リッチプレビュー & 右クリックコンテキストメニュー（Phase 8.6）：**
  * **明示的表示原則の厳守：** マウスホバーや左クリックによる意図しない自動抽出・自動表示を完全排除。一覧のファイル行を右クリックしてカスタムコンテキストメニューから「プレビューを表示」を選択した時のみ安全に発火。
  * **シンタックスハイライト（`syntaxHighlight.ts`）：** 外部依存なしの高速ハイライター。行番号付きテーブル構成で、Rust, TypeScript, JavaScript, Python, Shell, SQL, CSS, JSON, TOML, YAML 等のキーワード、文字列、数値、コメント、型、関数を Prism-dark 配色で着色。
  * **Markdown 簡易レンダリング：** 見出し（H1〜H4）、引用、箇条書き/番号付きリスト、インラインコード、コードブロック、太字、リンクを安全に HTML 変換・美麗レンダリング。
  * **HTML プレビュー：** サンドボックス iframe レンダリングと生ソースコード表示をワンクリックで切り替え可能なトグルボタンを実装。
  * **カスタムコンテキストメニュー（`ContextMenu.tsx`）：** サイバーダーク調の右クリックメニュー。プレビュー表示、ファイルパスコピー、作業領域投入、解除、削除を素早く実行可能。画面外はみ出し自動反転と Escape/外側クリック閉じる対応。
* **情報密度の 3 段階設定（Phase 8.7）：**
  * **3 段階の行高・サイズ構成：**
    * コンパクト（`compact`: 32px / アイコン 14px / フォント 11px）
    * 標準（`normal`: 42px / アイコン 16px / フォント 12px）
    * ゆったり（`spacious`: 52px / アイコン 20px / フォント 13px）
  * **システム連携（`density.ts`）：** CSS 変数（`--list-row-height`, `--list-icon-size`, `--list-font-size`, `--list-padding-y`）を動的更新し、`localStorage` に永続化。
  * **仮想スクロール（`@tanstack/react-virtual`）追従：** 動的な行高見積もりにより、設定変更時にも一覧のスクロール位置が崩れず瞬時に反映。
  * **設定モーダル（`SettingsModal.tsx`）：** 外観タブの先頭に直感的な 3 連ピルボタングループを設置。
* **ユーザー作成パイプライン & 設計書 8.3 必須安全フロー（Phase 8.8）：**
  * **設定ファイル & 設定メニューへのテンプレート設置（`config.template.toml`, `SettingsModal.tsx`）：**
    * `config.template.toml` に `[[custom_pipelines]]` の設定ブロック、`{input}`（作業コピー絶対パス）および `{output}`（出力先ステージングパス）プレースホルダーの解説、および WebP 変換・Exif 消去のサンプルを記載。
    * 設定モーダル（`SettingsModal.tsx`）に「カスタムパイプライン」タブを新設。ワンクリック TOML テンプレートコピー、プリセットテンプレート自動入力（WebP / ExifTool）、新規作成・編集フォーム、削除、`save_app_config` による即時永続化を実装。
  * **パイプラインモーダルへのユーザー作成項目追加（`PipelineModal.tsx`）：**
    * カテゴリピルに「ユーザー作成（`custom`）」を追加。
    * `AppConfig.custom_pipelines` から動的にプリセット一覧へ読み込み、ユーザー定義バッジ（`ShieldAlert` アイコン）とともに一覧表示。
  * **設計書 8.3 必須安全フローの完全強制（Mandatory Safety Flow）：**
    * ユーザー作成パイプライン選択時は直接実行ボタンを非表示にし、**2段階実行確認＋事前Dry-Run検証**を必須化：
      * **ステップ 1: [リスク説明]** 実行バイナリ名、引数展開パターン、対象ファイル数、および Linux Landlock LSM プロセス隔離下での原本保護制約を明示。
      * **第 1 回実行確認：** 「第1回実行確認：Dry-Run（事前検証）を開始」ボタンを押下。
      * **ステップ 2: [Dry-Run]** バックエンド `validate_custom_pipeline_dry_run` を呼び出し、PATH 内バイナリ解決、引数展開、出力ファイル名衝突、拡張子適合、空き容量（1.5倍）を事前検証。
      * **ステップ 3: [検証結果・リスク再提示]** 解決後絶対パス、展開後コマンドと予測成果物名のテーブル、衝突警告、合否判定バナーを表示し、原本保護制約を再提示。
      * **第 2 回実行確認：** 検証成功時のみ「第2回実行確認：Landlock 制限下で本実行」が活性化。
      * **ステップ 4: [本実行 (Landlock 隔離)]** 外部コマンドを原本書き込み遮断の sandbox 隔離下で実行。リアルタイム進捗バーおよびキャンセル処理に対応。
* **右クリックコンテキストメニューのポータル化・スマートフリップ & アクション列の余白適正化（Phase 8.9）：**
  * **コンテキストメニューの脱クリッピング（`ContextMenu.tsx`）：**
    * `ReactDOM.createPortal(..., document.body)` を導入し、親要素（`VirtualFileList` や `.browser-table-body`）の `overflow: hidden` や `transform: translateY(...)` による領域外クリッピングを完全解決。
    * 下部画面端（BottomTabPanel 付近）でのクリック時に上方向へ自動展開するスマート反転フリップ（`y - menuHeight`）および右端での左方向反転（`x - menuWidth`）を実装。`z-index: 99999` を適用し、どのパネルやダイアログよりも最前面に完全表示。
  * **ファイル名・INODE・アクションボタン（プレビュー／コピー）の配置余裕化（`App.css`, `VirtualFileList.tsx`）：**
    * アクション列（`.col-header.col-actions`, `.row-col-actions`）の幅を `32px` から `72px` へ倍増以上に拡大。
    * ボタン間に `gap: 6px` を付与し、ボタンサイズを `25px × 25px` に適正化。ホバースケールとプレビュー（ティール色）、ステージング（シアン色）の個別カラーリングを実装。
    * ファイル名・サイズ・日時・INODE カラムの各右側に `padding-right: 12px` のガター（余白）を統一設定し、隣接カラムとの詰まり感を完全に解消。
    * 表示密度（コンパクト／標準／ゆったり）に応じてボタンサイズ（22px / 25px / 28px）と列幅（62px / 72px / 82px）が滑らかに連動するレスポンシブスタイルを確立。
* **フォントファミリーとフォントサイズの設定メニュー（Phase 8.10）：**
  * **タイポグラフィ管理基盤（`src/utils/typography.ts`）：**
    * UI フォントファミリー（Sans-serif）、等幅コードフォント（Monospace / Paths）、基準フォントサイズ（11px〜18px / 79%〜129%）の統合管理インターフェースを実装。
    * **UI フォントプリセット（6種）：** 標準（Inter / Noto Sans）、モダン（Inter / Noto Sans JP）、システム標準（System Native）、和文ゴシック（BIZ UDP / 游ゴシック）、丸ゴシック（M PLUS Rounded）、カスタム指定（Custom）。
    * **等幅フォントプリセット（7種）：** 標準（JetBrains / Fira / Cascadia）、JetBrains Mono、Fira Code、Cascadia Code、和文等幅（BIZ UDGothic）、Symbols Nerd Font、カスタム指定（Custom）。
    * **カスタムフォント直接入力：** 任意のフォントファミリー名（ローカルインストール済みフォント）を入力可能。システムの安全なフォールバックスタックを自動補完。
    * **動的 CSS 変数注入 & 永続化：** `--font-sans`, `--font-mono`, `--app-base-font-size`, `--font-size-scale` を `:root` に即時反映し、`localStorage`（`cuddle-typography-settings`）に永続化。アプリ起動時に自動復元。
    * **ワンクリック初期化（Reset）：** 初期値（Inter 14px / JetBrains Mono）への瞬時リセット機能。
  * **設定モーダル UI 統合（`SettingsModal.tsx`）：**
    * 「外観（Appearance）」タブ内に専用カード（`.settings-typography-card`）を設置。
    * 基準フォントサイズスライダー（11px〜18px）およびクイック選択ピルボタン（11px, 12px, 13px, 14px, 15px, 16px, 18px）。
    * UI フォントおよび等幅フォントのプリセットピルボタンとカスタム入力欄。
    * リアルタイムライブプレビュー枠（UI テキスト見本 ＆ 等幅コード見本）を配備し、フォントファミリーやサイズの変更が即座に視覚確認可能。
  * **完全バイリンガル辞書同期（`typography.*`）：**
    * 日英（`ja/translation.json`, `en/translation.json`）の全ラベル・説明・プレースホルダーを完全同期。
  * **デザイン統一（白背景完全禁止）：**
    * 深海サイバーダーク背景（`#06090e`, `rgba(0,0,0,0.4)`）とサイバーティール（`#2dd4bf`）のアクセントを維持。
* **テスト検証：**
  * Vitest 50 件全件 100% パス（タイポグラフィ単体テスト `Typography.test.ts` を新設）、Cargo test 139 件全件 100% パス、TypeScript 型チェック / Vite ビルド完全成功。

---

## 9. Phase 9（未実装コア機能 & ソース公開配布体制）：完了

### 9.1 ユーザー合意事項（意思決定ゲート）

1. **配布方針（Distribution Policy）：** 初期リリースではライセンス管理等の煩雑さを避けるためバイナリパッケージ（deb, AppImage, Flatpak 等）の配布は行わず、**ソースコードのみを公開し、必須前提パッケージおよびビルド手順（README / BUILD）を完全に明記する**。
2. **エディタ連携リネーム（vidir型）仕様：**
   * **インターフェース：** アプリ内テキストエディタ ＋ 外部エディタ（`$VISUAL`, `$EDITOR`, VS Code, nano 等）呼び出しの両方に対応。
   * **フォーマット：** 設計書 9.4 準拠のシンプルなタブ区切りテキスト形式（`[item_id]<TAB>[ファイル名]`）。
   * **安全保護：** `item_id` で照合し、行削除は「ファイル削除」と解釈せず「変更なし（リネーム対象外）」として扱う。Two-Phase Rename およびロールバックジャーナル（UNDO）を完全適用。

### 9.2 実装内容（Phase 9.1: エディタ連携リネーム vidir型）：完了

* **バックエンドエンジン（`src-tauri/src/batch.rs`, `src-tauri/src/lib.rs`）：**
  * `generate_vidir_buffer`: 対象一覧から設計書 9.4 準拠のタブ区切りバッファを生成（安全上の注意事項ヘッダー付き）。
  * `parse_vidir_buffer`: 編集後のテキストをパースし、行削除されたアイテムは `OperationStatus::Unchanged` として安全に保持。重複衝突（`ConflictDuplicate`）、ディスク上既存衝突（`ConflictExistsOnDisk`）、不正ファイル名（`InvalidFilename`）を厳格に検出。
  * `launch_external_editor_vidir`: 一時ファイル（`~/.cache/Cuddle/vidir_<uuid>.tsv`）を生成し、`$VISUAL`, `$EDITOR`、または PATH 内のエディタ（`code --wait`, `gedit --wait`, `kate`, `gnome-text-editor`, `nano` 等）を直接プロセス起動。終了後に自動パース・一時ファイル安全消去。
  * Tauri IPC コマンド `generate_vidir_text`, `parse_vidir_text`, `launch_external_editor_vidir` を公開・登録。
  * Rust 単体テスト 2 件（`test_vidir_buffer_generation_and_parsing`, `test_vidir_collision_detection`）を追加し 100% パス。
* **フロントエンド UI 統合（`BulkRenameModal.tsx`, `App.css`）：**
  * モーダル上部に「ルール設定（Rules）」と「エディタ連携 (vidir型)」の切り替えタブバーを新設。
  * **アプリ内 vidir エディタ：** 行高・等幅フォント（`var(--font-mono)`）対応のテキストエリア、入力変更時のリアルタイム Dry-run 衝突検証。
  * **外部エディタ連携：** 「外部エディタで編集」ボタン（起動中スピナー表示、カスタムエディタコマンド入力枠、リロードボタン）。
  * **安全バッジ：** 「行を削除したファイルは変更なしとして扱われます（ファイルは削除されません）」を常時明示。
  * 下部 Dry-run サマリーバー・プレビュー差分テーブル・下部固定「一括リネームを実行」ボタンとの完全同期。
* **多言語同期（`ja/translation.json`, `en/translation.json`）：**
  * `bulk_rename.tab_rules`, `bulk_rename.tab_vidir`, `bulk_rename.vidir_*` の全キーを日英完全同期。
* **テスト検証：**
  * Vitest 51 件全件 100% パス（`BulkRenameModal.test.tsx` に vidir 連携テスト追加）、Cargo test 141 件全件 100% パス、TypeScript 型チェック / Vite ビルド完全成功。

### 9.3 実装内容（Phase 9.2: 一括属性変更 chmod 8進数＆シンボリック）：完了

* **バックエンドエンジン（`src-tauri/src/chmod.rs`, `src-tauri/src/lib.rs`）：**
  * `parse_octal_mode`: 3桁/4桁の8進数（例: `644`, `0755`, `4755`）を厳格にパース。
  * `parse_symbolic_mode`: POSIX chmod 互換のカンマ区切りシンボリック表現（例: `u+x`, `go-w`, `a+r`, `u=rwx,go=rx`、プレフィックス省略形式 `+x`, `-w`）を解釈。
  * `compute_target_mode`: 現在のファイル属性（`st_mode`）に対して演算子（`+`, `-`, `=`）を適用して変更後パーミッションを算出。
  * `has_special_permission_bits`: setuid (04000), setgid (02000), sticky (01000) の特殊ビットを判定し警告フラグをセット。
  * `generate_chmod_plan`: Dry-run 事前検証プラン（変更前/変更後の8進数・シンボリック文字列、変更判定、特殊ビット判定、警告情報）を生成。
  * `execute_chmod_plan`: 各アイテムの属性を変更し、失敗発生時は変更済みアイテムを逆順ロールバックして原子性を保護。完了後は SQLite WAL 操作履歴に記録。
  * Tauri IPC コマンド `generate_chmod_plan`, `execute_chmod_plan` を登録。
  * Rust 単体テスト 10 件を追加し、Cargo テスト全 151 件 100% パス。
* **フロントエンド UI 統合（`ChmodModal.tsx`, `App.css`, 各種パネル）：**
  * **モーダルダイアログ：** 8進数指定（`tab_octal`）とシンボリック指定（`tab_symbolic`）のタブ切り替え。
  * **クイックプリセットボタン：**
    * 8進数: `644`, `755`, `600`, `700`, `664`, `775`
    * シンボリック: `+x`, `-x`, `u+x`, `go-w`, `a+r`
  * **パーミッション・マトリクス：** 所有者・グループ・その他 × 読取・書込・実行（r/w/x）の双方向連動チェックボックス。
  * **特殊ビット警告バナー：** 特殊ビット（setuid/setgid/sticky）が含まれる場合にアンバー色警告バナーを表示。
  * **Dry-run 比較テーブル：** ファイル名、変更前（0644 / -rw-r--r--）、矢印、変更後（0755 / -rwxr-xr-x）、状態判定バッジ。
  * **パネル・コンテキストメニュー統合：**
    * `WorkspacePanel.tsx`: ヘッダーに「属性変更 (chmod)」ボタンを新設、右クリックメニューに項目追加。
    * `FileBrowserPanel.tsx`: 右クリックコンテキストメニューから個別・一括属性変更を起動可能。
* **多言語同期（`ja/translation.json`, `en/translation.json`）：**
  * `chmod` セクションの全キー（タイトル、タブ、プリセット、マトリクス、警告、ステータス、ボタン等）を日英完全同期。

### 9.4 実装内容（Phase 9.3: タイムスタンプ一括変更 & ワークスペース tar/tar.gz エクスポート）：完了

* **バックエンドエンジン（`src-tauri/src/timestamp.rs`, `src-tauri/src/export_tar.rs`, `src-tauri/src/lib.rs`）：**
  * **タイムスタンプ一括変更（`timestamp.rs`）：**
    * `filetime = "0.2"` および `kamadak-exif = "0.6"` による高精度タイムスタンプ制御。
    * **4つの変更モード（`TimestampModeSpec`）：**
      1. 現在時刻（`CurrentTime` / touch）
      2. 絶対日時指定（`Absolute` / `YYYY-MM-DD HH:mm:ss` 等の主要フォーマット対応）
      3. 相対時間シフト（`RelativeOffset` / 秒数指定、正数で未来、負数で過去へシフト）
      4. EXIF 撮影日時同期（`ExifSync` / 写真ファイルの Exif `DateTimeOriginal` メタデータを抽出し mtime へ反映）
    * `generate_timestamp_plan`: 変更前 mtime、変更予定 mtime、変更判定、警告を含む Dry-run 検証計画（`TimestampPlanDto`）を生成。
    * `execute_timestamp_plan`: 各アイテムの更新日時（mtime）を変更。エラー発生時は変更済みアイテムを逆順ロールバックして原子性を保護し、完了後に SQLite WAL 操作履歴（`OperationType::Timestamp`）へ自動記録。
    * Tauri IPC コマンド `generate_timestamp_plan`, `execute_timestamp_plan` を公開・登録。
  * **ワークスペース tar / tar.gz エクスポート（`export_tar.rs`）：**
    * 作業コピー領域（`workspace-temp/`）内のファイルを原本に影響を与えずアーカイブ出力。
    * **2つの出力形式（`TarExportFormat`）：** gzip 圧縮（`TarGz`: `.tar.gz` / 推奨）および非圧縮 POSIX tar（`Tar`: `.tar`）。
    * **3つのエクスポート対象（`TarExportScope`）：**
      1. 変更されたファイルのみ（`ModifiedOnly`: デフォルト推奨）
      2. ステージング全ファイル（`AllStaged`）
      3. 選択中のファイルのみ（`SelectedOnly`）
    * 出力先親ディレクトリの自動作成、元サイズとアーカイブサイズの集計、SQLite WAL 操作履歴（`OperationType::TarExport`）への自動記録。
    * Tauri IPC コマンド `export_workspace_tar` を公開・登録。
  * `ts-rs` による TypeScript 型自動生成（`TimestampModeSpec.ts`, `PlannedTimestampItemDto.ts`, `TimestampPlanDto.ts`, `TimestampExecutionResultDto.ts`, `TarExportFormat.ts`, `TarExportScope.ts`, `TarExportOptionsDto.ts`, `TarExportResultDto.ts`）。
  * 単体テスト 16 件を追加し、Cargo テスト全 167 件 100% パス。
* **フロントエンド UI 統合（`TimestampModal.tsx`, `TarExportModal.tsx`, `App.css`, 各種パネル）：**
  * **タイムスタンプモーダル（`TimestampModal.tsx`）：**
    * 4モード切り替えタブ（現在時刻 / 絶対指定 / 相対シフト / EXIF同期）。
    * クイックシフトボタン（+1時間, -1時間, +30分, -30分, +1日, -1日）。
    * Dry-run 比較テーブル（ファイル名、変更前日時、矢印、変更後日時、判定バッジ）。
    * 下部固定アクションバー、ロールバック通知、成功・エラーバナー。
  * **tar エクスポートモーダル（`TarExportModal.tsx`）：**
    * エクスポート対象カード選択（変更のみ / 全ステージング / 選択中のみ、各件数バッジ付き）。
    * アーカイブ形式カード選択（.tar.gz 推奨 / .tar、選択連動で出力ファイル名拡張子を自動同期）。
    * 出力先パス入力枠（再生成ボタン付き、親ディレクトリ自動作成注記）。
    * 完了時の成果物サイズ・元サイズ・ファイル数サマリー表示。
  * **デザイン統一（白背景完全禁止）：**
    * Midnight Teal / Cyber Dark テーマ統一（深海ダーク背景、ティール／シアン境界、ネオングリーン成功バナー）。
  * **パネル連携：**
    * `WorkspacePanel.tsx`: ヘッダーツールバーに「日時変更 (touch)」（`Clock` アイコン）および「tar エクスポート」（`FolderArchive` アイコン）を配備。
    * `VirtualFileList.tsx`: 右クリックコンテキストメニューに「日時変更 (touch)」項目を追加。
    * `FileBrowserPanel.tsx`: 右クリックコンテキストメニューから個別・一括日時変更を起動可能。
* **多言語同期（`ja/translation.json`, `en/translation.json`）：**
  * `timestamp` および `tar_export` セクションの全キー（タイトル、タブ、説明、プレースホルダー、ステータス、ボタン等）を日英完全同期。

### 9.6 実装内容（Phase 9.5: 原本への反映方式（方式 A / 方式 B）& 選択的 UNDO（Selective Undo））：完了

* **原本への反映方式（設計書 5.2 独立操作の徹底）：**
  * **2つの反映モード（`workspace::ReflectionMode`）：**
    * **方式 A（`Overwrite`）：** 原本と同一親ディレクトリ内に一時ファイル（`.cuddle_reflect_<uuid>.tmp`）を作成・コピー検証後、POSIX 原子的 `fs::rename(2)` により原本を安全に置換。
    * **方式 B（`BackupOrigin` / 推奨）：** 原本をタイムスタンプ付き `.origin`（例: `report_YYYYMMDD_HHMMSS.origin.pdf`、同名衝突時は連番サフィックス付与）へリネーム退避した上で作業成果物を配置。成果物配置に失敗した場合は退避先から自動ロールバックし原本の消失を物理的に防止。
  * **退避先パスのリアルタイムプレビュー：**
    * バックエンド IPC `get_origin_backup_preview` により、原本パスに応じた退避先予定パスを即時計算・返却。
  * **モーダル UI 統合（`BottomTabPanel.tsx`, `App.css`）：**
    * 原本反映確認モーダル内に方式 A / 方式 B のサイバーダーク調ラジオカード選択 UI を配備。
    * 方式 B 選択時に、退避先バックアップファイル名予定プレビュー（ネオンティール色）を明示。
    * 反映完了時、`ReflectExecutionResultDto`（更新後 `StagedItemDto`、バックアップパス `backup_path`、実行モード `mode`）を返却し、トースト通知とログ記録を同期。
* **選択的 UNDO（Selective Undo / 設計書 6.2）：**
  * **SQLite WAL 操作履歴との完全連携（`history.rs`, `batch.rs`, `lib.rs`）：**
    * 操作履歴テーブル `operation_history` から、`can_undo == true` の特定操作をピンポイントに取り消す統合 IPC コマンド `undo_history_operation(operation_id)` を実装。
    * **3大操作の個別逆変換ディスパッチ：**
      1. **一括リネーム（`BulkRename`）：** `batch::get_journal_manager().undo_batch_by_id(&operation_id)` により、対象バッチの変更操作を反転実行。
      2. **属性変更（`chmod`）：** `details` に記録された変更前モード（8進数）をパースし、`fs::set_permissions` で元のパーミッションへ即時復元。
      3. **タイムスタンプ変更（`touch_mtime`）：** `details` に記録された変更前ミリ秒タイムスタンプ（`[orig_ms:...]`）をパースし、`filetime::set_file_mtime` で元の更新日時へ高精度復元。
    * 取り消し完了後、SQLite WAL の該当レコードを `status = 'Undone'`, `can_undo = false` に更新し、取り消し履歴（`UndoBulkRename`, `UndoChmod`, `UndoTouchMtime`）を新たに記録。
  * **UI 統合（「操作履歴」タブ）：**
    * 履歴テーブルに「操作」カラム（`.th-actions`）を新設。
    * `can_undo == true` の各行に「元に戻す (UNDO)」（`RotateCcw` アイコン、ティール系）ボタンを配置。
    * クリック時に確認プロンプト（対象ファイル名表示）を経て、ローディングスピナーとともに非同期実行。
    * 実行完了後にファイル一覧・操作履歴を自動再走査・再取得。
* **多言語同期（`ja/translation.json`, `en/translation.json`）：**
  * `reflection.mode_*`, `reflection.backup_preview_label`, `history.col_actions`, `history.undo_*`, `history.type_undo_*` を日英完全同期。
* **テスト検証：**
  * Vitest 67 件全件 100% パス（原本反映方式 B / 方式 A 切り替えテスト、選択的 UNDO テストを追加）。
  * Cargo test 174 件全件 100% パス（`test_workspace_manager_reflect_mode_b_backup_origin`, `test_selective_undo_by_batch_id`, `test_undo_history_chmod_and_touch` を追加）。
  * TypeScript 型チェック / Vite 本番ビルド完全成功。

### 9.7 実装内容（Phase 9.6: デスクトップランチャー自動登録 & DE別インストーラー・アンインストーラー）：完了

* **マルチデスクトップ環境（DE）対応インストーラー（`scripts/`）：**
  * **CachyOS + COSMIC DE（Wayland）特化版（`scripts/install-cosmic.sh`）：**
    * CachyOS / Arch Linux および System76 COSMIC Desktop Environment（`cosmic-comp`, `cosmic-app-library`, `cosmic-launcher`）に最適化。
    * Wayland の `app_id` (`com.wammed.cuddle`) 照合、`StartupWMClass=com.wammed.cuddle`、`X-CosmicAppId=com.wammed.cuddle` を定義し、COSMIC App Library でのアイコン即時認識、検索インデックス化、ドックへのピン留めを完全保証。
  * **GNOME 特化版（`scripts/install-gnome.sh`）：**
    * GNOME 40〜47 (Wayland/X11) 向け。`StartupWMClass` によるウィンドウ集約と GNOME Shell 検索キャッシュの即時更新。
  * **KDE Plasma 特化版（`scripts/install-kde.sh`）：**
    * KDE Plasma 5/6 向け。`kbuildsycoca6` / `kbuildsycoca5` による Kickoff / KRunner / システム設定メニューキャッシュの再構築を実行。
  * **汎用自動検知インストーラー（`scripts/install.sh`）：**
    * `$XDG_CURRENT_DESKTOP`、`$XDG_SESSION_TYPE`、インストール先パスを自動判別。
    * `--user`（デフォルト：`~/.local/` 配下、root 不要）および `--system`（`/usr/local/` 配下、sudo 対応）をサポート。
    * 各解像度アイコン（32x32, 64x64, 128x128, 512x512 PNG）およびベクター SVG（`images/Cuddle-Icon.svg`）を `hicolor` アイコンツリーおよび `pixmaps` に自動配備。
    * FreeDesktop 規格準拠の `.desktop` ファイル（`com.wammed.cuddle.desktop` および互換 symlink `cuddle.desktop`）を生成・検証（`desktop-file-validate` 適合）。
    * `update-desktop-database` および `gtk-update-icon-cache` によるキャッシュ自動リフレッシュ。
  * **完全アンインストーラー（`scripts/uninstall.sh`）：**
    * 配備されたバイナリ、symlink、.desktop ファイル、全サイズアイコン、pixmaps をクリーンに除去し、キャッシュを再構築。
* **ドキュメント整備（日英完全同期）：**
  * `BUILD.ja.md` / `BUILD.md`: 第 8 項「デスクトップ環境へのインストール（ランチャー・アイコン登録）」を新設。
  * `README.ja.md` / `README.md`: クイックスタート内にワンライナーインストールコマンドを追記。

### 9.8 実装内容（全文検索インデックス明示的管理UI & 設計書 7.3 準拠 systemd 定期更新タイマー）：完了

* **検索インデックス明示的管理 UI（`SearchPanel.tsx`, `App.css`, 各種パネル）：**
  * 検索入力枠右隣に専用の「インデックス管理（Database）」ドロップダウンスイッチを新設。
  * **リアルタイム統計 ＆ 対象ディレクトリ表示：**
    * 登録済みドキュメント件数、インデックス状態（Ready）、対象ディレクトリ（現在フォルダパスまたは全範囲）のステータスピルを配備。
  * **4 つの明示的インデックス操作モード（`IndexOperationMode`）：**
    1. **差分・増分更新（`IncrementalUpdate` / 推奨）：** 既存ドキュメントの最終更新日時（mtime）と照合し、新規作成または変更されたファイルのみを抽出・登録。未変更ファイルは高速スキップ。
    2. **現在フォルダ登録（`IndexDirectory`）：** 現在開いているディレクトリ配下のテキスト・ソースコード・ドキュメントを再帰走査してインデックス化。
    3. **全インデックス再構築（`FullRebuild`）：** 既存インデックスを全消去した上でホームおよび対象領域をゼロから完全再走査・再構築。
    4. **インデックス初期化・全消去（`ResetClear`）：** Tantivy インデックス内の全登録ドキュメントを安全に消去（初期状態へリセット）。
  * **実行中フィードバック ＆ 完了サマリーバナー：**
    * 実行中はスピナー（`Loader2`）回転とボタン無効化による二重クリック防止。
    * 完了後は総処理件数、新規・更新件数、スキップ件数、エラー件数、所要時間（ミリ秒）を表示するサマリーバナーを即時描画。
  * **サイバーダーク統一：** 白背景を完全排除し、Dark Velvet Slate ＋ ネオングリーン／シアン／アンバー／ローズの色分けアイコンを採用。
* **バックエンドインデックス操作エンジン（`src-tauri/src/extractor.rs`, `src-tauri/src/search_fulltext.rs`）：**
  * `search_fulltext.rs`:
    * `clear_all_documents`: インデックス内の全ドキュメントを一括削除してコミット。
    * `get_total_documents`: 現在登録済みの有効ドキュメント数を高速取得。
    * `get_indexed_mtime`: Tantivy 0.26 クエリ（`item_id` / `raw_base64`）によるインデックス済み mtime 取得（増分判定用）。
    * `INIT_MUTEX` による二重チェックロッキングを導入し、並列テスト時の Tantivy `LockBusy` 競合を完全排除。
  * `extractor.rs`:
    * `run_index_operation`: `IncrementalUpdate`, `IndexDirectory`, `FullRebuild`, `ResetClear` の全モードを処理する統合ワーカー。
    * 未変更ファイルのスキップ、PDF 分離ワーカー（Landlock LSM ＋ メモリ上限 256 MiB）呼び出し、エラー許容継続処理。
* **設計書 7.3 準拠: アプリ停止中の定期更新（`src-tauri/src/periodic_index.rs`）：**
  * **カーネル排他ロック（`libc::flock`）による安全制御：**
    * `$XDG_DATA_HOME/Cuddle/cuddle.lock` に対する `flock(LOCK_EX | LOCK_NB)` を採用。
    * Cuddle GUI 起動中は静的 `GUI_LOCK` によりロックを保持。
    * バックグラウンド定期更新（`--index-update`）は起動時にロックを検査し、GUI が起動中であれば即座に終了コード 0 で正常終了（Tantivy インデックスの並行書き込み破損・ファイルロック競合を物理的に完全防止）。
  * **ヘッドレス CLI サブコマンド体系：**
    * `cuddle --index-update [--quiet]`: ヘッドレス増分インデックス更新の実行。
    * `cuddle --install-systemd-timer`: systemd ユーザーユニット（service / timer）の自動配置と有効化。
    * `cuddle --uninstall-systemd-timer`: systemd ユーザータイマーの停止・無効化・クリーン除去。
    * `cuddle --systemd-timer-status`: タイマーの登録状態、稼働状態、次回実行予定を出力。
  * **systemd ユーザーユニット（`systemd/cuddle-index.service`, `systemd/cuddle-index.timer`）：**
    * `Nice=19`, `IOSchedulingClass=idle` により作業中のマシンへの負荷をゼロ化。
    * `OnCalendar=*-*-* 02:30:00` かつ `Persistent=true` により、PC 起動時や深夜スリープ復帰時にも確実に増分更新を実行。
    * インストーラー（`scripts/install.sh`, `scripts/install-kde.sh`）で自動配備・有効化、アンインストーラー（`scripts/uninstall.sh`）で自動削除。
* **Tauri IPC コマンド & TypeScript 型同期：**
  * `get_index_stats`, `run_index_operation`, `get_systemd_timer_status`, `install_systemd_timer`, `uninstall_systemd_timer` を公開・登録。
  * `ts-rs` による TypeScript 型自動生成（`IndexStatsDto.ts`, `RunIndexOptions.ts`, `IndexOperationMode.ts`, `IndexOperationResultDto.ts`, `SystemdTimerStatusDto.ts`）。
* **多言語同期（`ja/translation.json`, `en/translation.json`）：**
  * `search.index_*` の全キー（タイトル、各操作モード名、説明、統計、ボタン、サマリーラベル等）を日英完全同期。
* **テスト検証：**
  * Vitest 71 件全件 100% パス（`SearchPanel.test.tsx` 新設）。
  * Cargo test 184 件全件 100% パス（`test_run_index_operations_workflow`, `test_app_lock_exclusive` 等を追加）。
  * Cargo clippy 警告ゼロ、TypeScript 型チェック / Vite ビルド完全成功。

---

## 10. 開発上の重要制約（Guardrails）

* **root 権限実行の禁止：** アプリは常に非特権一般ユーザー権限で動作させる（権限昇格 `sudo`/`pkexec` は行わない）。
* **原本保護の徹底：** 原本ファイルに対する直接編集・インプレース更新は行わない。すべての変更は `workspace-temp/` 内の作業コピーに対して行い、明示的な反映操作でのみ置換する。
* **安全警告の非表示化禁止：** 破壊的操作、権限不整合、原本競合に関する重大警告は、ユーザー設定でオフにできないようにハードコードする。
* **任意シェル文字列実行の排除：** コマンド実行時は `sh -c` や `bash -c` を一切使用せず、引数配列による直接プロセス起動（Landlock 適用下）を行う。
* **単一言語ハードコードの排除：** UI 上に直接日本語または英語文字列をハードコードせず、必ず `t('key')` 経由で取得する。

---

## 11. 次期実装タスク・バックログ（Phase 10〜14）

統合基本設計書（[`FileWorkbench-Design-ver0.9.ja.md`](file:///home/susie/git/wammed/Cuddle/FileWorkbench-Design-ver0.9.ja.md)）と現行コードベースの網羅的照合に基づき、未実装または差異のある 18 項目をトリアージし、[`NEXT_IMPLEMENTATION_TASKS.ja.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.ja.md) として策定しました。

### フェーズ構成と推奨着手フロー

1. **Phase 10: UX ＆ 操作性クイックウィン ＆ 視覚的超リッチ化 【実装完了・手動テスト承認済み】**
   * 【2-2】ペイン幅・配置のセッション間復元（`react-resizable-panels` / `localStorage`）: **実装完了・実機検証済み**
   * 【2-1】ブラウザー ⇔ ワークスペース間のドラッグ＆ドロップ（D&D ステージング）: **実装完了・実機検証済み**
   * 【2-4】パイプラインログの表示モード切り替え（`all` / `hidden` / `file_only` フィルタ）: **実装完了・実機検証済み**
   * 【2-5】拡張子別リッチアイコン（Rust, TS, Python, メディア等の個別配色・Lucideアイコン）: **実装完了・実機検証済み**
   * 【2-6】光と被写界深度の強化（インナーリムライト ＆ 3層多層シャドウ）: **実装完了・実機検証済み**
   * 【2-7】マウス追従スポットライト ＆ スプリング触感フィードバック: **実装完了・実機検証済み**
   * 【2-8】ストレージ容量ゲージ ＆ パーミッション/サイズ視覚化（インラインサイズヒートバー、パーミッションチップ）: **実装完了・実機検証済み**
   * 【2-9】等幅数字（Tabular Figures）＆ インタラクティブ・ブレッドクラム: **実装完了・実機検証済み**
   * 【2-10】アンビエント・オーロラメッシュ背景 ＆ 微細グレイン（30色テーマ完全同期 ＆ 高輝度オーブ）: **実装完了・実機検証済み**
2. **Phase 11: 安全設計 ＆ 原本反映・リカバリーの洗練 【実装完了・手動テスト承認済み】**
   * 【3-2】原本競合時の別名保存（例: `.edit.pdf`）の UI 提示と安全退避フロー: **実装完了・実機手動テスト承認済み**
     * バックエンド `reflect_staged_item_as_copy`（サフィックス指定・衝突時連番付与）、`get_alternate_copy_preview` を実装。
     * 原本を一切改変せずに成果物を安全保存。原本反映モーダルで競合検知時に専用別名保存UIを表示。
   * 【3-3】バックアップ保持数（既定 5 件、上限 10 件）の管理と古い世代の削除 UI: **実装完了・実機手動テスト承認済み**
     * バックエンド `config.rs`（`backup_retention_count`）、`backup.rs`（`list_backups`, `delete_backup`, `prune_old_backups`, `restore_workspace_backup`）を実装。
     * `ExitBackupModal` に保持ポリシーインジケーター、自動プルーニングチェック、アーカイブ一覧＆個別削除ボタンを実装。`SettingsModal` に世代数スライダーを追加。
     * スタートページ（`StartPage.tsx`）に「バックアップから再開」スイッチおよびアーカイブ選択モーダルを追加。
   * 【4-3】Landlock 非対応環境での起動ブロック / 同意ダイアログ: **実装完了・実機手動テスト承認済み**
     * `CriticalSecurityModal` を新設。Landlock LSM 非対応またはカーネル制約時に全画面警告を表示し、同意または終了を強制。選択記憶トグルを実装。
   * 【終了時安全インターセプト & ヘッダー終了ボタン】: **実装完了・実機手動テスト承認済み**
     * ウィンドウ右上×ボタン押下時の `onCloseRequested` インターセプトによる Zero Data Loss 保証、およびヘッダー終了ボタンを実装。
3. **Phase 12: パイプライン実行・外部連携の柔軟性拡張 【実装完了・実機手動テスト待ち】**
   * 【4-2】ユーザー定義カスタムパイプラインでの `allowed_read_paths` 指定（Landlock 連携）: **実装完了・検証済み**
     * `CustomPipelineDefDto` に `allowed_read_paths: Vec<String>` を追加。パイプライン実行時および事前 Dry-Run 時に Landlock LSM の allow_read に動的登録。
     * `SettingsModal` に設定欄・Landlock Read バッジ、`PipelineModal` のリスク説明に許可パスを表示。
   * 【4-1】入力方式（`batch`, `stdin`）と型付きプレースホルダー（`{InputList}`, `{WorkspaceDirectory}`）: **実装完了・検証済み**
     * `ExecutionInputMode`（`PerFile`, `BatchList`, `StandardInput`）を導入。
     * `expand_command_arguments` による `{input}`, `{output}`, `{InputList}`（安全なスプラット展開）, `{WorkspaceDirectory}` のプレースホルダー置換。
     * `SandboxedExecutor` での子プロセス `stdin` ストリーミング送信と EOF クローズ処理。
     * `SettingsModal` に入力方式選択とワンクリック挿入ピルを追加。
   * 【2-3】OS / デスクトップ環境ネイティブ通知の抽象化（FreeDesktop D-Bus 通知）: **実装完了・検証済み**
     * `notify.rs` を新設し、`notify-send`（COSMIC / GNOME / KDE 準拠）による `notify_desktop` および Tauri コマンド `notify_user` を実装。
     * パイプライン完了時にデスクトップ通知を自動発行。フロントエンド共通ヘルパー `notifyUser` を整備。
4. **Phase 13: 検索・インデックス・本文抽出の高度化 【次回セッション着手予定】**
   * 【1-4】Markdown（`pulldown-cmark`）/ HTML の構文解析本文抽出
   * 【1-3】第 3 段階: Office 文書（`.docx`, `.xlsx` 等）の本文抽出（メモリ制限付き分離ワーカー）
   * 【1-2】形態素解析（IPADIC）＋ n-gram 併用ハイブリッド検索
   * 【1-1】インデックス差分更新・修復キュー（Repair Queue）
   * 【1-5】除外設定ごとのインデックス保持方針（メタデータのみ / ファイル名のみ）
5. **Phase 14: アーキテクチャ再検討・超大規模対応**
   * 【3-1】確認レベル 4 段階設定（標準/厳格への集約維持を検討）
   * 【3-4】4 テーブルクラッシュリカバリー（現行 SQLite WAL + BatchJournal の堅牢性追認）
   * 【5-1】TOML 10 セクション構成（現行 4 セクション集約モデル維持を検討）
   * 【6-1】大量転送時の IPC チャンク分割ストリーミング（ベンチマーク計測後の判断）

詳細な実装方針、影響ファイル、テスト計画は [`NEXT_IMPLEMENTATION_TASKS.ja.md`](file:///home/susie/git/wammed/Cuddle/NEXT_IMPLEMENTATION_TASKS.ja.md) を参照してください。

# FileWorkbench (Cuddle) 開発引き継ぎ書

<p align="center">
  <a href="FileWorkbench-Handover.en.md">English</a> | <strong>日本語</strong>
</p>

**文書作成日：** 2026-10-25  
**ステータス：** Phase 5 完了、Phase 6 着手準備完了  
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

### 7.3 Phase 7.3 の目的と設計方針（リアルタイム進捗 UI & キャンセル制御）
* **非同期ストリーミング通知と UI 統合：**
  * 各ファイルの処理状況（待機・実行中・成功・失敗）をリアルタイムに進捗プログレスバーで表示。
  * ユーザーによる途中キャンセル機能の実装。

---

## 7. 開発上の重要制約（Guardrails）

* **root 権限実行の禁止：** アプリは常に非特権一般ユーザー権限で動作させる（権限昇格 `sudo`/`pkexec` は行わない）。
* **原本保護の徹底：** 原本ファイルに対する直接編集・インプレース更新は行わない。すべての変更は `workspace-temp/` 内の作業コピーに対して行い、明示的な反映操作でのみ置換する。
* **安全警告の非表示化禁止：** 破壊的操作、権限不整合、原本競合に関する重大警告は、ユーザー設定でオフにできないようにハードコードする。
* **任意シェル文字列実行の排除：** コマンド実行時は `sh -c` や `bash -c` を一切使用せず、引数配列による直接プロセス起動（Landlock 適用下）を行う。
* **単一言語ハードコードの排除：** UI 上に直接日本語または英語文字列をハードコードせず、必ず `t('key')` 経由で取得する。

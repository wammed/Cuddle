<p align="center">
  <img src="images/Cuddle-Banner.jpeg" alt="Cuddle Banner" width="100%" />
</p>

<p align="center">
  <a href="README.md">English</a> | <strong>日本語</strong>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/プラットフォーム-Linux%20(Wayland%20%2F%20X11)-teal.svg" alt="Platform" />
  <img src="https://img.shields.io/badge/セキュリティ-Linux%20Landlock%20LSM-emerald.svg" alt="Security" />
  <img src="https://img.shields.io/badge/基盤-Tauri%202%20%2B%20Rust-orange.svg" alt="Engine" />
  <img src="https://img.shields.io/badge/ライセンス-MIT-blue.svg" alt="License" />
</p>

---

# Cuddle - 原本保護型ファイル作業環境 (File Workbench)

**Cuddle** は、Linux デスクトップ環境（Wayland, COSMIC DE, GNOME, KDE）向けに設計された、高信頼原本保護型ファイル作業環境・高速検索アプリケーションです。

従来のファイルマネージャーやユーティリティは、原本ファイルを直接インプレースで編集・変更します。万一、一括リネームで同名衝突が発生したり、外部スクリプトの引数が狂ったり、予期せぬエラーが起きた場合、原本データは破損・喪失してしまいます。**Cuddle は、原本保護の絶対原則によりこのリスクを根本から解決します。**

---

## 主要な基本方針 & コア思想

1. **原本保護の絶対性（Original Protection）：**  
   原本ファイルを**決して直接触りません**。すべての変換、リネーム、属性変更、外部コマンド実行は、隔離された一時作業領域（`workspace-temp/`）内の作業コピーに対してのみ安全に適用されます。
2. **作業確定と原本反映の完全分離（Reflection Isolation）：**  
   作業成果物は、ユーザーが明示的に「原本へ変更を反映」を選択した場合にのみ、2段階確認と TOCTOU（Time-of-Check to Time-of-Use）競合検知を経て安全に原本へ反映されます。
3. **Linux Landlock LSM によるカーネルレベル隔離（Landlock Sandbox）：**  
   外部ツール（ImageMagick, FFmpeg, cwebp, pdftotext 等）やユーザー定義パイプラインの実行時、Rust 起動直前に Linux Landlock ルールセットを適用。作業領域外および `$HOME` への書き込みを Linux カーネルレベルで物理的に遮断します。
4. **Reflink (CoW) による瞬時ステージング：**  
   原本と同一ファイルシステムボリューム上の一時領域を優先利用し、Linux `ioctl(FICLONE)` によるコピーオンライトを活用して、容量消費ゼロ・ミリ秒単位で作業コピーを生成します。
5. **決定論的アトミックロールバック（RollbackAll）：**  
   一括リネームや属性・日時変更は厳格な Two-Phase 方式で処理されます。1件でもエラーが発生した場合は全体を中断し、変更前状態へ自動的に完全ロールバックします。
6. **SQLite WAL 永続操作履歴 & アンドゥ：**  
   ステージング、解除、リネーム、属性変更、パイプライン等の全操作は ACID SQLite WAL ジャーナルに自動永続化され、ワンクリックでの一括アンドゥ（取り消し）に対応しています。
7. **フォント同梱ゼロ（Zero Font Bundling） & サイバーダーク美学：**  
   巨大なフォントアセットを同梱せず、OS の洗練されたフォントスタック（Inter, Noto Sans CJK JP, JetBrains Mono, Symbols Nerd Font）を動的フォールバック参照。白背景を完全根絶した 30 色の Midnight Teal / Cyber Dark カラーテーマを搭載しています。
8. **100% 日英完全バイリンガル同期：**  
   日本語と英語の表示言語を、アプリ再起動不要で即時に動的切り替え可能です。

---

## 機能マトリクス

| 機能 | 詳細説明 |
| :--- | :--- |
| **6領域アダプティブレイアウト** | 検索結果、ディレクトリツリー、ファイルブラウザ、作業領域、詳細インスペクター、下部タブ（差分・履歴・Git・ログ）を美しく統合。 |
| **仮想スクロール描画** | `@tanstack/react-virtual` により、最大 10,000 件のファイル一覧を 60fps で滑らかに描画。3 段階の情報密度（コンパクト／標準／ゆったり）に対応。 |
| **一括リネーム & Unicode 正規化** | 正規表現置換、Unicode 正規化（NFC/NFD/NFKC/NFKD）、全角半角変換、連番付与、日時スタンプ、およびリアルタイム Dry-Run 衝突検証テーブル。 |
| **vidir 型エディタ連携** | 対象ファイル名をタブ区切りテキストとしてアプリ内エディタまたは外部エディタ（`$EDITOR`, VS Code, nano 等）で一括編集。行削除はファイル削除ではなく安全に「変更なし」として維持。 |
| **30種の定義済みパイプライン** | 画像軽量化（WebP, OGP, アイコン, Retina）、動画・音声マスタリング、PDF 二値化・統合、ログ機密マスキング、クリーンソース配布パッケージ生成など。 |
| **ユーザー作成パイプライン** | 設計書 8.3 の必須安全フローに準拠。リスク説明、事前 Dry-Run 検証、および Landlock LSM 隔離実行による 2 段階確認を強制。 |
| **ファイル属性の一括変更 (chmod)** | 8進数（644, 755等）およびシンボリック（`u+x`, `go-w`等）指定。3x3 権限マトリクスと特殊ビット（setuid/setgid/sticky）警告を完備。 |
| **タイムスタンプの一括変更 (touch)** | 現在時刻更新（touch）、絶対日時指定、相対時間シフト（+/- 秒・時間・日）、および写真 EXIF 撮影日時の mtime 自動同期。 |
| **ワークスペース tar エクスポート** | 作業中のファイルを原本を汚染せず `.tar` または `.tar.gz` 形式でアーカイブ出力（変更のみ／全ファイル／選択中のみ）。 |
| **日本語形態素 全文検索** | Tantivy 0.26 ＋ Lindera 形態素解析（`ipadic`）による高精度な日本語本文検索とリアルタイム抜粋スニペット表示。 |
| **plocate 高速インデックス検索** | ミリ秒単位のファイル名検索。plocate 未利用時は `walkdir` 直接走査へ安全に自動フォールバック。 |
| **リッチプレビュー** | 10種以上の主要言語のシンタックスハイライト、Markdown レンダリング、サンドボックス iframe による HTML プレビュー。 |
| **Git ステータスダッシュボード** | リポジトリ稼働状況、ブランチ、Upstream 同期情報（Ahead / Behind）、最新コミット、および変更ファイルの内訳を表示。 |

---

## クイックスタート & ビルド方法

ソースコードから Cuddle をビルド・実行する手順：

```bash
# リポジトリのクローン
git clone https://github.com/wammed/Cuddle.git
cd Cuddle

# フロントエンド依存関係のインストール
pnpm install

# 開発モードでの起動（Tauri 2 デスクトップシェルが起動します）
pnpm run tauri dev

# 本番バイナリのビルド & デスクトップランチャー登録（アイコン・.desktop自動配備）
pnpm run tauri build
./scripts/install-cosmic.sh   # CachyOS + COSMIC DE（推奨）
# または ./scripts/install.sh  # GNOME / KDE / XFCE 汎用自動検知
```

詳細な Linux 前提環境、システムパッケージ、推奨外部ツール、デスクトップ環境別のインストール設定については、**[BUILD.ja.md](BUILD.ja.md)**（または英語版 **[BUILD.md](BUILD.md)**）を参照してください。

---

## 関連ドキュメント

* **[BUILD.ja.md](BUILD.ja.md)** / **[BUILD.md](BUILD.md)**: ビルド・環境構築・依存パッケージ導入ガイド
* **[FileWorkbench-Design-ver0.9.ja.md](FileWorkbench-Design-ver0.9.ja.md)** / **[FileWorkbench-Design-ver0.9.en.md](FileWorkbench-Design-ver0.9.en.md)**: 統合基本設計書（残課題解決・バイリンガル仕様確定版）
* **[FileWorkbench-Handover.ja.md](FileWorkbench-Handover.ja.md)** / **[FileWorkbench-Handover.en.md](FileWorkbench-Handover.en.md)**: 開発引き継ぎ書・実装進捗詳細記録
* **[IP_COMPLIANCE.ja.md](IP_COMPLIANCE.ja.md)** / **[IP_COMPLIANCE.md](IP_COMPLIANCE.md)**: ブランドアイコン・知的財産コンプライアンス記録

---

## ライセンス

本ソフトウェアは [MIT License](LICENSE) の下で公開されています。  
Copyright (c) 2026 wammed and Cuddle Contributors.

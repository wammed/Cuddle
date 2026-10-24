<p align="center">
  <img src="images/Cuddle-Banner.jpeg" alt="Cuddle バナー" width="100%" />
</p>

<p align="center">
  <img src="images/Cuddle-Icon.svg" alt="Cuddle アイコン" width="96" height="96" />
</p>

# Cuddle

<p align="center">
  <strong>File Workbench</strong><br />
  <em>原本を大切に抱きしめ、絶対に傷つけない高信頼 Linux ファイルワークベンチ</em>
</p>

<p align="center">
  <a href="README.md">English</a> | <strong>日本語</strong> | <a href="IP_COMPLIANCE.ja.md">知的財産コンプライアンス記録</a>
</p>

---

## 1. プロジェクト概要

**Cuddle** は、Linux デスクトップ（CachyOS、COSMIC DE、Wayland、HiDPI 環境）向けに設計された、原本保護型ファイル管理・全文検索・一括バッチ処理ワークベンチです。

**「原本を抱きしめる（Cuddle）」** というコア思想のもと、外部ツール実行や一括リネームなどのすべての作業を完全に隔離された作業コピー（`workspace-temp/`）内で行い、ユーザーが明示的に確定するまで原本を物理的に一切変更しない安全機構を備えています。

---

## 2. コア設計思想・基本方針

1. **原本保護の絶対性：**  
   原本ファイルは常に読み取り専用（Read-Only）として扱います。すべての編集・フォーマット変換・バッチ処理は独立した作業コピーに対してのみ実行されます。
2. **確定操作と原本反映の完全分離：**  
   パイプライン実行や一括操作の中で原本を直接インプレース更新することは絶対にありません。成果物確認後、ユーザーの明示的な確定操作によってのみ原本へ反映されます。
3. **カーネルレベル隔離（Linux Landlock LSM）：**  
   外部プロセス（ImageMagick、ffmpeg、cwebp等）の起動直前に Linux Landlock ルールセットを適用し、原本や `$HOME` への書き込みをカーネルレベルで物理的に遮断します。
4. **決定論的ロールバック（RollbackAll）：**  
   一括操作中に 1 件でもエラーが発生した場合、全体を失敗とみなし、開始前の完全な初期状態へと確実に復元します。
5. **二重キーによる非 UTF-8 ファイル名耐性：**  
   Rust 内部では `[u8]` の生バイト列を一貫保持し、IPC 境界では Base64 キー（`raw_base64`）と安全な表示名（`display_name`）を併載。UUID で追跡することで文字化けによるクラッシュを完全排除します。
6. **完全動的バイリンガル対応（日英両言語）：**  
   アプリ内のすべてのメニュー、警告、メッセージ、ダイアログは日本語と英語の完全バイリンガル仕様であり、再起動不要で即時切り替えが可能です。

---

## 3. UI/UX コンセプト & デザインシステム

### 「シンプルだけど超リッチな見た目」（Dark Velvet & Radiant Protection）
- **無駄を削ぎ落とした洗練（シンプル）：** 開発者やパワーユーザーが作業に没頭できるよう、情報の優先度を整理したクリーンで高密度なレイアウト。
- **奥行きと質感（超リッチ）：** 半透明のグラスモーフィズムサーフェス（`backdrop-filter: blur(16px)`）、極細のヘアラインボーダー（`1px rgba(255, 255, 255, 0.08)`）、呼吸するように発光する安全インジケーター（ソフトグロー）。
- **デスクトップとの調和：** モダンな Linux デスクトップ環境と調和するダークチャコール基調に、保護（シアン）、安全（エメラルド）、警告（アンバー）の発光アクセントを配置。

### タイポグラフィ & フォント選定（Nerd Font 前提条件仕様）
- **フォントバイナリの非同梱（Zero Font Bundling）：** Cuddle はアプリパッケージ内に大容量のフォントバイナリ（Nerd Font等）を**一切同梱しません**。軽量・安全でライセンスに配慮した配布を実現します。
- **前提条件としてのシステムフォントスタック（Pre-required）：**
  - **UI（サンセリフ）：** `Inter`、システムフォント、高品質和文フォント（`Noto Sans CJK JP` / `Noto Sans JP` 等）。
  - **コード・パス（モノスペース）：** `JetBrains Mono`、`Fira Code`、`Noto Sans Mono CJK JP`。
  - **アイコン・グリフ（Nerd Font フォールバック）：** `Symbols Nerd Font`、`JetBrainsMono Nerd Font`。
- **堅牢なフォールバック設計：** システムに Nerd Font がインストールされていない環境でも、CSS フォールバックスタックと内蔵の `lucide-react` SVG ベクターアイコンにより、文字化け（豆腐化）や表示崩れを一切発生させず、100% 美しい UI 描画を保証します。

---

## 4. 動作前提条件（Prerequisites）

- **OS：** Linux カーネル 5.13 以上（Landlock LSM ABI V1〜V5 対応。実機検証: CachyOS / COSMIC DE）
- **Node.js：** v20 以上 / pnpm v9 以上
- **Rust：** 1.80 以上（Edition 2021）
- **推奨フォント（任意・前提条件）：**  
  ファイルグリフやステータスアイコンを Nerd Font で楽しむ場合、システムのパッケージマネージャー等から `ttf-nerd-fonts-symbols` や各種 Nerd Font を事前にインストールしておくことを推奨します（例: Arch/CachyOS の場合 `sudo pacman -S ttf-nerd-fonts-symbols`）。

---

## 5. 開発 & テスト手順

```bash
# フロントエンド依存関係のインストール
pnpm install

# フロントエンドテストの実行 (Vitest)
pnpm test

# Rust 単体テスト & Landlock 遮断テストの実行
cd src-tauri && cargo test && cd ..

# 開発用デスクトップアプリの起動
pnpm tauri dev
```

---

## 6. ライセンス & コンプライアンス

- **コード：** MIT License に基づき配布されます。詳細は [LICENSE](LICENSE) を参照してください。
- **アイコン & ブランド意匠：** AI 支援生成の来歴および知的財産デューデリジェンスの記録は [IP_COMPLIANCE.ja.md](IP_COMPLIANCE.ja.md) を参照してください。

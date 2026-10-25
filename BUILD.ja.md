# Cuddle - ビルド & 開発環境構築ガイド

<p align="center">
  <a href="BUILD.md">English</a> | <strong>日本語</strong>
</p>

本書では、**Cuddle** を Linux 上でソースコードからビルド・実行するために必要なシステム依存パッケージ、ツールチェーン、およびビルド手順を解説します。

---

## 1. 動作対象システム & 検証済み環境

* **対象 OS:** Linux x86_64
* **動作検証済み環境：**
  * **CachyOS / Arch Linux** (Wayland, COSMIC DE / GNOME / KDE) - *主要リファレンスデスクトップ*
  * **Ubuntu 24.04 LTS / 22.04 LTS**
  * **Fedora 40 / 39**
  * **Debian 12 (Bookworm)**
* **カーネル要件：**
  * Linux カーネル **5.13 以降**、かつ Landlock LSM が有効であること（`CONFIG_SECURITY_LANDLOCK=y`）。
  * 実行中のカーネルで Landlock が有効か確認するコマンド：
    ```bash
    cat /sys/kernel/security/lsm
    # 出力リストに landlock が含まれていれば利用可能です
    ```

---

## 2. ツールチェーンの前提条件

以下の開発ツールをあらかじめインストールしてください：

1. **Rust ツールチェーン（1.77 以降）：**
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   rustc --version
   cargo --version
   ```
2. **Node.js（v20 LTS 以降）& pnpm：**
   ```bash
   # ディストリビューションのパッケージマネージャー、または nvm/fnm で Node.js を導入
   node --version

   # pnpm (v9 以降) の有効化
   corepack enable
   # または
   npm install -g pnpm
   pnpm --version
   ```

---

## 3. ネイティブシステム依存パッケージ (Tauri 2 Linux)

Tauri 2 の動作には WebKitGTK および基本的な開発ヘッダーが必要です：

### Arch Linux / CachyOS
```bash
sudo pacman -S --needed \
  base-devel \
  webkit2gtk-4.1 \
  openssl \
  curl \
  wget \
  file \
  squashfs-tools \
  libappindicator-gtk3 \
  librsvg
```

### Ubuntu / Debian (22.04 / 24.04)
```bash
sudo apt update
sudo apt install -y \
  build-essential \
  pkg-config \
  libwebkit2gtk-4.1-dev \
  libappindicator3-dev \
  librsvg2-dev \
  patchelf \
  libssl-dev \
  libasound2-dev
```

### Fedora
```bash
sudo dnf install -y \
  @development-tools \
  pkg-config \
  webkit2gtk4.1-devel \
  openssl-devel \
  libappindicator-gtk3-devel \
  librsvg2-devel
```

---

## 4. パイプライン用 推奨外部 CLI ツール（任意）

Cuddle は、データのクレンジング、ログのマスキング、JSON の軽量化、TSV/CSV 相互変換、SRT 字幕のタイムコードシフト等の純 Rust 内蔵高速変換機能を備えています。

画像・音声・動画・PDF の高度なプリセットを利用する場合は、以下の外部 CLI ツールをインストールしてください：

```bash
# Arch / CachyOS
sudo pacman -S --needed imagemagick ffmpeg qpdf poppler perl-image-exiftool plocate

# Ubuntu / Debian
sudo apt install -y imagemagick ffmpeg qpdf poppler-utils libimage-exiftool-perl plocate

# Fedora
sudo dnf install -y ImageMagick ffmpeg qpdf poppler-utils perl-Image-ExifTool plocate
```

*※ Cuddle は起動時にこれらのツールを自動検出します。未インストールのツールがあってもアプリは安全に稼働し、純 Rust プリセットは即時利用可能です。*

---

## 5. ローカル開発 & 起動手順

1. **リポジトリをクローン：**
   ```bash
   git clone https://github.com/wammed/Cuddle.git
   cd Cuddle
   ```

2. **フロントエンド依存パッケージをインストール：**
   ```bash
   pnpm install
   ```

3. **開発モードで起動：**
   ```bash
   pnpm run tauri dev
   ```
   *Vite 開発サーバーと Tauri 2 デスクトップシェルが自動起動します。*

---

## 6. テストスイートの実行

### バックエンド Rust 単体・統合テスト（174件）
```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

### フロントエンド Vitest 単体テスト（67件）
```bash
pnpm test -- --run
```

### TypeScript 型チェック & Vite 本番ビルド検証
```bash
pnpm run build
```

---

## 7. 本番バイナリのビルド

最適化されたネイティブバイナリをビルドする手順：

```bash
pnpm run tauri build
```

生成された実行可能バイナリは以下に出力されます：
```
src-tauri/target/release/fileworkbench
```

直接実行して動作を確認できます：
```bash
./src-tauri/target/release/fileworkbench
```

---

## 8. デスクトップ環境へのインストール（ランチャー・アイコン登録）

Cuddle には、ビルド済みバイナリの配備、FreeDesktop 規格準拠の `.desktop` ランチャー登録、各解像度（32/64/128/512px + SVG）アイコンのインストール、およびデスクトップキャッシュ更新を自動化するスクリプトが用意されています。

### 8.1 CachyOS + COSMIC DE（Wayland）向け最適化インストール（推奨）

CachyOS および System76 COSMIC Desktop Environment（`cosmic-comp`, `cosmic-app-library`, `cosmic-launcher`）に特化したプリセットです。Wayland の `app_id` (`com.wammed.cuddle`) 照合、ドックへのピン留め、および検索インデックスの即時更新を保証します：

```bash
# ユーザーローカルインストール（推奨：~/.local 配下、root 不要）
./scripts/install-cosmic.sh

# システム全体インストール（/usr/local 配下、要 sudo）
sudo ./scripts/install-cosmic.sh --system
```

* **反映確認:** `Super` キーを押して COSMIC App Library を開き、`Cuddle` を検索または右クリックで「Pin to Dock」できます。

### 8.2 代表的なデスクトップ環境向けプリセット

お使いの DE に応じた専用スクリプトも用意されています：

* **GNOME (Wayland / X11):**
  ```bash
  ./scripts/install-gnome.sh
  ```
  `StartupWMClass=com.wammed.cuddle` によるウィンドウ集約と GNOME Shell 検索インデックスの即時反映を行います。

* **KDE Plasma 5 / 6 (Wayland / X11):**
  ```bash
  ./scripts/install-kde.sh
  ```
  `kbuildsycoca6` / `kbuildsycoca5` によるシステム設定・Kickoff / KRunner メニューキャッシュの再構築を実行します。

* **汎用自動検知インストーラー (XFCE, Cinnamon, MATE, その他):**
  ```bash
  ./scripts/install.sh
  ```
  実行中の `$XDG_CURRENT_DESKTOP` や `$XDG_SESSION_TYPE` を自動解析し、最適なパラメータでインストールします。

### 8.3 アンインストール手順

登録されたランチャー、全解像度アイコン、バイナリを完全に削除し、キャッシュを更新します：

```bash
# ユーザーローカル環境から削除
./scripts/uninstall.sh

# システム全体から削除
sudo ./scripts/uninstall.sh --system
```

---

## 9. トラブルシューティング & FAQ

### Landlock ABI 警告が表示される場合
* **症状：** UI のセキュリティ状態に「お使いのカーネルでは Landlock LSM がサポートされていません」と表示される。
* **解決策：** カーネルで Landlock が有効（`CONFIG_SECURITY_LANDLOCK=y`）になっていること、およびカーネル起動オプション（GRUB や systemd-boot）に `lsm=...,landlock` が含まれていることを確認してください。Landlock が無効であっても、Cuddle は一時領域での作業コピー保護を継続して安全に動作します。

### 非 UTF-8 ファイル名の文字化け
* Cuddle は二重キー管理アーキテクチャ（`raw_base64` ＋ `display_name`）を採用しており、不正なバイト列を含むファイル名であってもアプリがクラッシュすることなく安全に扱えます。

### フォント・アイコンの表示
* Cuddle は肥大化を避けるためフォントファイルをバイナリに同梱していません。アイコンやグリフが文字化けする場合は、OS に標準的なフォントを導入してください：
  * Arch: `sudo pacman -S ttf-nerd-fonts-symbols noto-fonts-cjk`
  * Ubuntu: `sudo apt install fonts-noto-cjk fonts-font-awesome`


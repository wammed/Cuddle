# Cuddle ライセンス・知的財産（IP）管理ポータル

<p align="center">
  <a href="README.md">English</a> | <strong>日本語</strong> | <a href="../../README.ja.md">🏠 ルート README</a> | <a href="IP_COMPLIANCE.ja.md">🛡️ アイコン IP 記録</a>
</p>

本書は、**Cuddle (File Workbench)** プロジェクトにおけるサードパーティライセンス、知的財産権（IP）、特許リスクの管理方針、および将来実施予定の包括的ライセンス監査（Comprehensive License Audit）に向けたドキュメンテーションインデックスです。

---

## 1. ライセンス管理基本方針

Cuddle は、開発者やパワーユーザーが安心して利用・改変・再配布できるよう、**MIT License**（極めて寛容なパーミッシブライセンス）の下で公開されています。

本体コードベースおよび配布バイナリにおいて、以下の原則を厳格に維持します：

1. **MIT 互換性の維持:**
   * Cuddle 本体に静的リンク・コンパイルされるすべての Rust クレートおよび npm パッケージは、MIT、Apache-2.0、BSD-2/3-Clause 等の寛容ライセンスに限定する。
   * 強コピーレフト（GPLv2/v3、AGPL）のライブラリは、本体バイナリに直接組み込まない。
2. **特許リスク・パテントプールの回避:**
   * HEVC (H.265) 等、特許ロイヤリティやライセンスプールが複雑に絡む技術のデコーダ／エンコーダは、Cuddle 配布バイナリに一切内包（バンドル）しない。
3. **外部ツール委譲（Dynamic CLI Delegation）の徹底:**
   * 必要な外部処理（HEIC 変換、高度なメタデータ処理等）は、システム側に存在するツール（ImageMagick、FFmpeg、ExifTool 等）を検知し、Linux Landlock LSM サンドボックス隔離下で外部プロセスとして呼び出す方式を採用する。
4. **完全バイリンガル同期:**
   * すべてのライセンス・IP 関連ドキュメントは、英語および日本語の双方で維持する。

---

## 2. ライセンス・IP ドキュメント一覧

| ドキュメント | 言語 | 内容・対象スコープ |
| :--- | :--- | :--- |
| [`IMAGE_PREVIEW_IP_AUDIT.ja.md`](IMAGE_PREVIEW_IP_AUDIT.ja.md) | 日本語 | **画像プレビュー機能（ネイティブ・RAW・HEIC・PSD 等）のライセンス・特許リスク分析と設計根拠、監査チェックリスト** |
| [`IMAGE_PREVIEW_IP_AUDIT.en.md`](IMAGE_PREVIEW_IP_AUDIT.en.md) | English | Image Preview & Media Processing IP / Patent Audit Record & Checklist |
| [`IP_COMPLIANCE.ja.md`](IP_COMPLIANCE.ja.md) | 日本語 | アプリケーションアイコンおよびブランド意匠の由来・デューデリジェンス記録 |
| [`IP_COMPLIANCE.md`](IP_COMPLIANCE.md) | English | Application Icon & Brand IP Compliance and Due Diligence Record |
| [`THIRD-PARTY-LICENSES.md`](THIRD-PARTY-LICENSES.md) | 英語 | サードパーティ依存関係・ライセンス監査証跡 |
| [`../../LICENSE`](../../LICENSE) | 英語 | Cuddle プロジェクト公式 MIT ライセンス条文 |

---

## 3. 全機能完了時の包括的ライセンス監査計画

Cuddle の全計画フェーズ（Phase 14）完了時、リリースに向けた最終関門として以下のライセンス監査工程を実施します：

1. **Rust 依存関係の機械的監査:**
   * `cargo-deny` による全推移的依存クレートのライセンス分類（Permissive, Copyleft, Unknown）照合。
   * LGPL / GPL の静的リンク混入ゼロの検証。
2. **Frontend (npm) 依存関係の機械的監査:**
   * `license-checker-rseidelsohn` 等を用いた npm 依存関係の抽出と NOTICE ファイル生成。
3. **特許回避アーキテクチャの動作検証:**
   * HEIC/HEIF 等の外部委譲がスタンドアロンバイナリに不要なコードを含んでいないことのバイナリシンボル検査（`nm`, `objdump`, `readelf`）。

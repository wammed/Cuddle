import { invoke } from "@tauri-apps/api/core";
import type { AppConfig } from "../types/AppConfig";
import type { EditorSnippetDto } from "../types/EditorSnippetDto";

export type SnippetLanguageKey = string;

export interface SnippetLanguageOption {
  key: string;
  label: string;
  badge: string;
  syntaxLang: string;
}

export const KNOWN_LANGUAGES_MAP: Record<
  string,
  { label: string; badge: string; syntaxLang: string; commentPrefix: string }
> = {
  bash: { label: "Bash", badge: "BASH", syntaxLang: "shell", commentPrefix: "# " },
  fish: { label: "Fish", badge: "FISH", syntaxLang: "shell", commentPrefix: "# " },
  zsh: { label: "Zsh", badge: "ZSH", syntaxLang: "shell", commentPrefix: "# " },
  jsonc: { label: "JSON / JSONC", badge: "JSONC", syntaxLang: "json", commentPrefix: "// " },
  toml: { label: "TOML", badge: "TOML", syntaxLang: "toml", commentPrefix: "# " },
  yaml: { label: "YAML", badge: "YAML", syntaxLang: "yaml", commentPrefix: "# " },
  lua: { label: "Lua", badge: "LUA", syntaxLang: "lua", commentPrefix: "-- " },
  plaintext: { label: "Plain Text", badge: "TXT", syntaxLang: "plaintext", commentPrefix: "# " },
  python: { label: "Python", badge: "PY", syntaxLang: "python", commentPrefix: "# " },
  rust: { label: "Rust", badge: "RS", syntaxLang: "rust", commentPrefix: "// " },
  typescript: { label: "TypeScript", badge: "TS", syntaxLang: "typescript", commentPrefix: "// " },
  javascript: { label: "JavaScript", badge: "JS", syntaxLang: "javascript", commentPrefix: "// " },
  sql: { label: "SQL", badge: "SQL", syntaxLang: "sql", commentPrefix: "-- " },
  html: { label: "HTML", badge: "HTML", syntaxLang: "html", commentPrefix: "<!-- " },
  css: { label: "CSS", badge: "CSS", syntaxLang: "css", commentPrefix: "/* " },
  markdown: { label: "Markdown", badge: "MD", syntaxLang: "markdown", commentPrefix: "# " },
};

export const BASE_SNIPPET_LANGUAGES: SnippetLanguageOption[] = [
  { key: "bash", label: "Bash", badge: "BASH", syntaxLang: "shell" },
  { key: "fish", label: "Fish", badge: "FISH", syntaxLang: "shell" },
  { key: "zsh", label: "Zsh", badge: "ZSH", syntaxLang: "shell" },
  { key: "jsonc", label: "JSON / JSONC", badge: "JSONC", syntaxLang: "json" },
  { key: "toml", label: "TOML", badge: "TOML", syntaxLang: "toml" },
  { key: "yaml", label: "YAML", badge: "YAML", syntaxLang: "yaml" },
  { key: "lua", label: "Lua", badge: "LUA", syntaxLang: "lua" },
  { key: "plaintext", label: "Plain Text", badge: "TXT", syntaxLang: "plaintext" },
];

export const SNIPPET_LANGUAGES = BASE_SNIPPET_LANGUAGES;

/**
 * Returns available languages dynamically merged from base languages, known languages,
 * and any custom language found in registered snippets (e.g. python, rust, etc.).
 */
export function getAvailableSnippetLanguages(
  snippets: EditorSnippetDto[] = []
): SnippetLanguageOption[] {
  const seen = new Set<string>();
  const result: SnippetLanguageOption[] = [];

  // Always include the 8 base required languages first
  for (const base of BASE_SNIPPET_LANGUAGES) {
    seen.add(base.key);
    result.push(base);
  }

  // Include any other languages present in the snippets collection (user-added or config.toml)
  for (const s of snippets) {
    const key = s.language?.trim().toLowerCase();
    if (key && !seen.has(key)) {
      seen.add(key);
      if (KNOWN_LANGUAGES_MAP[key]) {
        result.push({
          key,
          label: KNOWN_LANGUAGES_MAP[key].label,
          badge: KNOWN_LANGUAGES_MAP[key].badge,
          syntaxLang: KNOWN_LANGUAGES_MAP[key].syntaxLang,
        });
      } else {
        const capitalized = key.charAt(0).toUpperCase() + key.slice(1);
        result.push({
          key,
          label: capitalized,
          badge: key.slice(0, 4).toUpperCase(),
          syntaxLang: "plaintext",
        });
      }
    }
  }

  return result;
}

/**
 * Gets comment syntax helpers for a given language key
 */
export function getCommentPrefixForLanguage(langKey: string): {
  prefix: string;
  check: string;
  regex: RegExp;
} {
  const lower = langKey.toLowerCase();
  if (lower === "lua" || lower === "sql") {
    return { prefix: "-- ", check: "--", regex: /^(\s*)--\s?/ };
  }
  if (
    lower === "jsonc" ||
    lower === "rust" ||
    lower === "typescript" ||
    lower === "javascript"
  ) {
    return { prefix: "// ", check: "//", regex: /^(\s*)\/\/\s?/ };
  }
  return { prefix: "# ", check: "#", regex: /^(\s*)#\s?/ };
}

/**
 * 40 Default Pre-configured Snippets (5 per language)
 * Mirrors Rust backend's get_default_editor_snippets() in config.rs
 */
export const DEFAULT_EDITOR_SNIPPETS: EditorSnippetDto[] = [
  // --- Bash (5 presets) ---
  {
    id: "bash-path",
    language: "bash",
    label: "+ PATH 追加",
    snippet: 'export PATH="$HOME/.local/bin:$PATH"',
    description: "~/.local/bin を PATH に追加",
  },
  {
    id: "bash-alias",
    language: "bash",
    label: "+ alias 定義",
    snippet: "alias ll='ls -la --color=auto'",
    description: "詳細ファイルリストのエイリアス",
  },
  {
    id: "bash-env",
    language: "bash",
    label: "+ export 環境変数",
    snippet: 'export EDITOR="nvim"',
    description: "デフォルトエディタ等の環境変数を設定",
  },
  {
    id: "bash-mkcd",
    language: "bash",
    label: "+ mkcd 関数",
    snippet: "mkcd() {\n  mkdir -p \"$1\" && cd \"$1\"\n}",
    description: "ディレクトリを作成して即座に移動する関数",
  },
  {
    id: "bash-ps1",
    language: "bash",
    label: "+ カラーPS1 プロンプト",
    snippet: 'export PS1="\\[\\e[32m\\]\\u@\\h\\[\\e[0m\\]:\\[\\e[34m\\]\\w\\[\\e[0m\\]\\$ "',
    description: "ユーザー名と作業ディレクトリを色分け表示",
  },

  // --- Fish (5 presets) ---
  {
    id: "fish-path",
    language: "fish",
    label: "+ fish_add_path",
    snippet: "fish_add_path $HOME/.local/bin",
    description: "Fish の安全な PATH 追加コマンド",
  },
  {
    id: "fish-alias",
    language: "fish",
    label: "+ alias 定義",
    snippet: "alias ll 'ls -la'",
    description: "Fish エイリアス構文",
  },
  {
    id: "fish-env",
    language: "fish",
    label: "+ set -gx 環境変数",
    snippet: "set -gx EDITOR nvim",
    description: "グローバルエクスポート環境変数",
  },
  {
    id: "fish-abbr",
    language: "fish",
    label: "+ abbr 略称展開",
    snippet: "abbr -a gco git checkout",
    description: "入力時に自動展開される略称コマンド",
  },
  {
    id: "fish-function",
    language: "fish",
    label: "+ mkcd 関数",
    snippet: "function mkcd --description 'ディレクトリ作成と移動'\n  mkdir -p $argv[1]; and cd $argv[1]\nend",
    description: "Fish 構文でのディレクトリ作成＆移動関数",
  },

  // --- Zsh (5 presets) ---
  {
    id: "zsh-path",
    language: "zsh",
    label: "+ path 配列追加",
    snippet: 'path=("$HOME/.local/bin" $path); export PATH',
    description: "Zsh 固有の path 配列による PATH 設定",
  },
  {
    id: "zsh-alias",
    language: "zsh",
    label: "+ alias 定義",
    snippet: "alias ll='ls -la'",
    description: "Zsh エイリアス定義",
  },
  {
    id: "zsh-autosuggest",
    language: "zsh",
    label: "+ 補完提案ハイライト",
    snippet: "export ZSH_AUTOSUGGEST_HIGHLIGHT_STYLE='fg=60'",
    description: "zsh-autosuggestions のテキスト配色指定",
  },
  {
    id: "zsh-history",
    language: "zsh",
    label: "+ 履歴共有設定",
    snippet: "HISTSIZE=10000\nSAVEHIST=10000\nsetopt SHARE_HISTORY",
    description: "セッション間でコマンド履歴をリアルタイム共有",
  },
  {
    id: "zsh-mkcd",
    language: "zsh",
    label: "+ mkcd 関数",
    snippet: "mkcd() {\n  mkdir -p \"$1\" && cd \"$1\"\n}",
    description: "ディレクトリ作成＆移動関数",
  },

  // --- JSONC (5 presets) ---
  {
    id: "jsonc-config",
    language: "jsonc",
    label: "+ 設定オブジェクト",
    snippet: '{\n  // アプリケーション設定\n  "version": "1.0.0",\n  "enabled": true\n}',
    description: "コメント付き基本設定スキーマ",
  },
  {
    id: "jsonc-keybinding",
    language: "jsonc",
    label: "+ キーバインド定義",
    snippet: '{\n  "key": "ctrl+shift+p",\n  "command": "workbench.action.showCommands"\n}',
    description: "VS Code / Cuddle 形式のショートカット定義",
  },
  {
    id: "jsonc-tsconfig",
    language: "jsonc",
    label: "+ tsconfig オプション",
    snippet: '"compilerOptions": {\n  "target": "ES2022",\n  "module": "ESNext",\n  "strict": true\n}',
    description: "TypeScript strict コンパイラ設定",
  },
  {
    id: "jsonc-task",
    language: "jsonc",
    label: "+ タスク定義",
    snippet: '{\n  "label": "build",\n  "type": "shell",\n  "command": "pnpm build"\n}',
    description: "自動化タスク定義",
  },
  {
    id: "jsonc-env",
    language: "jsonc",
    label: "+ env オブジェクト",
    snippet: '"env": {\n  "NODE_ENV": "development",\n  "PORT": 3000\n}',
    description: "環境変数ブロック",
  },

  // --- TOML (5 presets) ---
  {
    id: "toml-package",
    language: "toml",
    label: "+ [package] セクション",
    snippet: '[package]\nname = "my-project"\nversion = "0.1.0"\nedition = "2021"',
    description: "Cargo / プロジェクト基本メタデータ",
  },
  {
    id: "toml-dependencies",
    language: "toml",
    label: "+ [dependencies] 定義",
    snippet: '[dependencies]\nserde = { version = "1.0", features = ["derive"] }\ntokio = { version = "1.0", features = ["full"] }',
    description: "クレート依存関係",
  },
  {
    id: "toml-settings",
    language: "toml",
    label: "+ [settings] 設定群",
    snippet: '[settings]\nenabled = true\ntimeout_seconds = 30\nlog_level = "info"',
    description: "汎用アプリケーション設定セクション",
  },
  {
    id: "toml-profile",
    language: "toml",
    label: "+ [profile.release] 最適化",
    snippet: "[profile.release]\nopt-level = 3\nlto = true\ncodegen-units = 1",
    description: "リリースビルド最適化設定",
  },
  {
    id: "toml-table-array",
    language: "toml",
    label: "+ [[servers]] テーブル配列",
    snippet: '[[servers]]\nhost = "127.0.0.1"\nport = 8080',
    description: "リスト形式のテーブル配列設定",
  },

  // --- YAML (5 presets) ---
  {
    id: "yaml-service",
    language: "yaml",
    label: "+ compose サービス",
    snippet: "services:\n  app:\n    image: alpine:latest\n    restart: unless-stopped\n    volumes:\n      - ./:/workspace",
    description: "Docker Compose 基本サービス定義",
  },
  {
    id: "yaml-action",
    language: "yaml",
    label: "+ GitHub Actions ステップ",
    snippet: "- name: Setup Node.js\n  uses: actions/setup-node@v4\n  with:\n    node-version: 20",
    description: "CI ワークフロー設定ステップ",
  },
  {
    id: "yaml-configmap",
    language: "yaml",
    label: "+ ConfigMap 定義",
    snippet: "apiVersion: v1\nkind: ConfigMap\nmetadata:\n  name: app-config\ndata:\n  APP_ENV: production",
    description: "Kubernetes ConfigMap マニフェスト",
  },
  {
    id: "yaml-env",
    language: "yaml",
    label: "+ environment 変数群",
    snippet: "environment:\n  NODE_ENV: production\n  PORT: 8080\n  DEBUG: \"false\"",
    description: "環境変数リスト",
  },
  {
    id: "yaml-matrix",
    language: "yaml",
    label: "+ matrix マトリクスビルド",
    snippet: "matrix:\n  os: [ubuntu-latest, macos-latest]\n  node: [18, 20]",
    description: "複数環境テスト用のマトリクス定義",
  },

  // --- Lua (5 presets) ---
  {
    id: "lua-keymap",
    language: "lua",
    label: "+ vim.keymap.set",
    snippet: "vim.keymap.set('n', '<leader>ff', '<cmd>Telescope find_files<cr>', { desc = 'Find files' })",
    description: "Neovim キーマップ設定",
  },
  {
    id: "lua-opt",
    language: "lua",
    label: "+ vim.opt 基本設定",
    snippet: "vim.opt.number = true\nvim.opt.relativenumber = true\nvim.opt.shiftwidth = 2\nvim.opt.expandtab = true",
    description: "行番号とインデント幅設定",
  },
  {
    id: "lua-fn",
    language: "lua",
    label: "+ local function 定義",
    snippet: 'local function greet(name)\n  return "Hello, " .. (name or "world")\nend',
    description: "関数定義テンプレート",
  },
  {
    id: "lua-lazy",
    language: "lua",
    label: "+ lazy.nvim プラグイン",
    snippet: 'return {\n  "nvim-treesitter/nvim-treesitter",\n  build = ":TSUpdate",\n  config = function()\n    require("nvim-treesitter.configs").setup({ highlight = { enable = true } })\n  end,\n}',
    description: "Lazy プラグインマネージャ定義",
  },
  {
    id: "lua-module",
    language: "lua",
    label: "+ モジュールテーブル (M)",
    snippet: 'local M = {}\nM.setup = function(opts)\n  M.config = vim.tbl_deep_extend("force", { enabled = true }, opts or {})\nend\nreturn M',
    description: "標準的な Neovim Lua モジュール構造",
  },

  // --- Plain Text (5 presets) ---
  {
    id: "txt-todo",
    language: "plaintext",
    label: "+ [ ] TODO 項目",
    snippet: "- [ ] TODO: ",
    description: "未完了チェックボックスタスク",
  },
  {
    id: "txt-done",
    language: "plaintext",
    label: "+ [x] 完了項目",
    snippet: "- [x] DONE: ",
    description: "完了済みチェックボックスタスク",
  },
  {
    id: "txt-header",
    language: "plaintext",
    label: "+ セクション区切り線",
    snippet: "# ------------------------------------------------------------------------------\n# ",
    description: "長文メモ用のアスキー境界ヘッダー",
  },
  {
    id: "txt-timestamp",
    language: "plaintext",
    label: "+ 日付スタンプ",
    snippet: "[=== YYYY-MM-DD HH:MM ===]",
    description: "作業ログの記録ヘッダー",
  },
  {
    id: "txt-license",
    language: "plaintext",
    label: "+ 著作権ライセンス表記",
    snippet: "Copyright (c) 2026. All rights reserved.",
    description: "ソースコードや文書の権利表記",
  },
];

/**
 * Suggests an initial snippet language based on filename
 */
export function detectInitialSnippetLanguage(filename: string): SnippetLanguageKey {
  const lower = filename.toLowerCase();

  if (
    lower === ".bashrc" ||
    lower === ".bash_profile" ||
    lower === ".bash_aliases" ||
    lower.endsWith(".sh") ||
    lower.endsWith(".bash")
  ) {
    return "bash";
  }

  if (lower === "config.fish" || lower.endsWith(".fish")) {
    return "fish";
  }

  if (lower === ".zshrc" || lower === ".zshenv" || lower === ".zprofile" || lower.endsWith(".zsh")) {
    return "zsh";
  }

  if (lower.endsWith(".json") || lower.endsWith(".jsonc")) {
    return "jsonc";
  }

  if (
    lower.endsWith(".toml") ||
    lower === ".gitconfig" ||
    lower === ".editorconfig" ||
    lower.endsWith(".ini") ||
    lower.endsWith(".conf") ||
    lower.endsWith(".cfg")
  ) {
    return "toml";
  }

  if (lower.endsWith(".yaml") || lower.endsWith(".yml")) {
    return "yaml";
  }

  if (lower.endsWith(".lua")) {
    return "lua";
  }

  if (lower.endsWith(".py") || lower.endsWith(".pyw")) {
    return "python";
  }

  if (lower.endsWith(".rs")) {
    return "rust";
  }

  if (lower.endsWith(".ts") || lower.endsWith(".tsx")) {
    return "typescript";
  }

  if (lower.endsWith(".js") || lower.endsWith(".jsx") || lower.endsWith(".mjs")) {
    return "javascript";
  }

  if (lower.endsWith(".sql")) {
    return "sql";
  }

  if (lower.endsWith(".md") || lower.endsWith(".markdown")) {
    return "markdown";
  }

  return "plaintext";
}

/**
 * Loads snippets from AppConfig (config.toml via Tauri invoke) with fallback
 */
export async function loadSnippets(): Promise<EditorSnippetDto[]> {
  try {
    const cfg = await invoke<AppConfig>("get_app_config");
    if (cfg && Array.isArray(cfg.snippets) && cfg.snippets.length > 0) {
      return cfg.snippets;
    }
  } catch (err) {
    console.error("Failed to load snippets from app_config:", err);
  }
  return DEFAULT_EDITOR_SNIPPETS;
}

/**
 * Saves snippets to AppConfig (config.toml via Tauri invoke)
 */
export async function saveSnippets(newSnippets: EditorSnippetDto[]): Promise<void> {
  try {
    const cfg = await invoke<AppConfig>("get_app_config");
    const updatedCfg: AppConfig = {
      ...cfg,
      snippets: newSnippets,
    };
    await invoke("save_app_config", { config: updatedCfg });
  } catch (err) {
    console.error("Failed to save snippets to app_config:", err);
    throw err;
  }
}


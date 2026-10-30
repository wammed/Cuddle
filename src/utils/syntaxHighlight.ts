// src/utils/syntaxHighlight.ts
// Fast, lightweight, pure-TypeScript syntax highlighter and Markdown renderer
// Zero external heavy dependencies, 100% offline & secure.

export type SupportedLanguage =
  | "markdown"
  | "html"
  | "rust"
  | "typescript"
  | "javascript"
  | "python"
  | "json"
  | "toml"
  | "yaml"
  | "shell"
  | "css"
  | "sql"
  | "lua"
  | "plaintext";

export function detectLanguage(filename: string): SupportedLanguage {
  const lower = filename.toLowerCase();

  // Special config dotfiles & filenames (Shell / Config)
  if (
    lower === ".bashrc" ||
    lower === ".bash_profile" ||
    lower === ".bash_logout" ||
    lower === ".bash_aliases" ||
    lower === ".zshrc" ||
    lower === ".zprofile" ||
    lower === ".zshenv" ||
    lower === ".zlogin" ||
    lower === ".profile" ||
    lower === ".env" ||
    lower.startsWith(".env.") ||
    lower === "dockerfile" ||
    lower === "makefile"
  ) {
    return "shell";
  }

  if (lower === ".gitconfig" || lower === ".editorconfig") {
    return "toml";
  }

  const ext = filename.split(".").pop()?.toLowerCase() || "";
  switch (ext) {
    case "md":
    case "markdown":
      return "markdown";
    case "html":
    case "htm":
    case "svg":
    case "xml":
      return "html";
    case "rs":
      return "rust";
    case "ts":
    case "tsx":
      return "typescript";
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return "javascript";
    case "py":
      return "python";
    case "json":
      return "json";
    case "toml":
    case "ini":
    case "conf":
    case "cfg":
      return "toml";
    case "yaml":
    case "yml":
      return "yaml";
    case "sh":
    case "bash":
    case "zsh":
      return "shell";
    case "css":
    case "scss":
    case "less":
      return "css";
    case "sql":
      return "sql";
    case "lua":
      return "lua";
    default:
      return "plaintext";
  }
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#039;");
}

const KEYWORDS: Record<SupportedLanguage, Set<string>> = {
  rust: new Set([
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false",
    "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut",
    "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
    "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn",
  ]),
  typescript: new Set([
    "break", "case", "catch", "class", "const", "continue", "debugger", "default",
    "delete", "do", "else", "enum", "export", "extends", "false", "finally", "for",
    "function", "if", "import", "in", "instanceof", "new", "null", "return", "super",
    "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while",
    "with", "as", "implements", "interface", "let", "package", "private", "protected",
    "public", "static", "yield", "any", "boolean", "constructor", "declare", "get",
    "module", "require", "number", "set", "string", "symbol", "type", "from", "of",
  ]),
  javascript: new Set([
    "break", "case", "catch", "class", "const", "continue", "debugger", "default",
    "delete", "do", "else", "export", "extends", "false", "finally", "for",
    "function", "if", "import", "in", "instanceof", "new", "null", "return", "super",
    "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while",
    "with", "let", "static", "yield", "from", "of", "async", "await",
  ]),
  python: new Set([
    "and", "as", "assert", "async", "await", "break", "class", "continue", "def",
    "del", "elif", "else", "except", "False", "finally", "for", "from", "global",
    "if", "import", "in", "is", "lambda", "None", "nonlocal", "not", "or", "pass",
    "raise", "return", "True", "try", "while", "with", "yield", "self",
  ]),
  shell: new Set([
    "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until",
    "do", "done", "in", "function", "select", "time", "echo", "export", "local",
    "return", "exit", "source", "alias",
  ]),
  sql: new Set([
    "SELECT", "FROM", "WHERE", "INSERT", "INTO", "UPDATE", "DELETE", "JOIN",
    "LEFT", "RIGHT", "INNER", "OUTER", "ON", "GROUP", "BY", "ORDER", "HAVING",
    "LIMIT", "OFFSET", "AS", "AND", "OR", "NOT", "NULL", "IS", "IN", "LIKE",
    "CREATE", "TABLE", "DROP", "ALTER", "PRIMARY", "KEY", "FOREIGN",
  ]),
  css: new Set([
    "important", "media", "keyframes", "font-face", "import", "charset",
  ]),
  json: new Set(["true", "false", "null"]),
  toml: new Set(["true", "false"]),
  yaml: new Set(["true", "false", "null", "yes", "no"]),
  lua: new Set([
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function",
    "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then",
    "true", "until", "while",
  ]),
  html: new Set(),
  markdown: new Set(),
  plaintext: new Set(),
};

/**
 * Tokenize and highlight code for dark theme rendering
 */
export function highlightCode(code: string, lang: SupportedLanguage): string {
  const lines = code.split("\n");
  const keywords = KEYWORDS[lang] || new Set();

  return lines
    .map((line, lineIndex) => {
      let highlighted = "";
      let i = 0;

      while (i < line.length) {
        // Line comments (//, #, --)
        if (
          (lang !== "python" && lang !== "shell" && line.slice(i, i + 2) === "//") ||
          ((lang === "python" || lang === "shell" || lang === "yaml" || lang === "toml") && line[i] === "#") ||
          ((lang === "sql" || lang === "lua") && line.slice(i, i + 2) === "--")
        ) {
          highlighted += `<span class="syn-comment">${escapeHtml(line.slice(i))}</span>`;
          break;
        }

        // Strings: "..." or '...' or `...`
        if (line[i] === '"' || line[i] === "'" || line[i] === "`") {
          const quote = line[i];
          let end = i + 1;
          while (end < line.length && line[end] !== quote) {
            if (line[end] === "\\" && end + 1 < line.length) {
              end += 2;
            } else {
              end++;
            }
          }
          if (end < line.length) end++; // include closing quote
          highlighted += `<span class="syn-string">${escapeHtml(line.slice(i, end))}</span>`;
          i = end;
          continue;
        }

        // Numbers: digits
        if (/\d/.test(line[i]) && (i === 0 || /[^a-zA-Z0-9_]/.test(line[i - 1]))) {
          let end = i;
          while (end < line.length && /[\d\.xXa-fA-F_]/.test(line[end])) {
            end++;
          }
          highlighted += `<span class="syn-number">${escapeHtml(line.slice(i, end))}</span>`;
          i = end;
          continue;
        }

        // HTML tags if lang is HTML
        if (lang === "html" && line[i] === "<") {
          let end = line.indexOf(">", i);
          if (end !== -1) {
            end++;
            highlighted += `<span class="syn-tag">${escapeHtml(line.slice(i, end))}</span>`;
            i = end;
            continue;
          }
        }

        // Identifiers & Keywords
        if (/[a-zA-Z_]/.test(line[i])) {
          let end = i;
          while (end < line.length && /[a-zA-Z0-9_]/.test(line[end])) {
            end++;
          }
          const word = line.slice(i, end);
          if (keywords.has(word) || (lang === "sql" && keywords.has(word.toUpperCase()))) {
            highlighted += `<span class="syn-keyword">${escapeHtml(word)}</span>`;
          } else if (end < line.length && line[end] === "(") {
            highlighted += `<span class="syn-fn">${escapeHtml(word)}</span>`;
          } else if (/^[A-Z][a-zA-Z0-9_]*$/.test(word)) {
            highlighted += `<span class="syn-type">${escapeHtml(word)}</span>`;
          } else {
            highlighted += escapeHtml(word);
          }
          i = end;
          continue;
        }

        // Punctuation & other characters
        highlighted += escapeHtml(line[i]);
        i++;
      }

      const lineNum = `<span class="code-line-num font-mono">${lineIndex + 1}</span>`;
      return `<div class="code-line">${lineNum}<span class="code-line-content font-mono">${highlighted || "&nbsp;"}</span></div>`;
    })
    .join("");
}

/**
 * Lightweight safe Markdown renderer into HTML
 */
export function renderMarkdown(md: string): string {
  const lines = md.split("\n");
  let inCodeBlock = false;
  let codeBlockLang: SupportedLanguage = "plaintext";
  let codeBuffer: string[] = [];
  const htmlOut: string[] = [];

  for (let idx = 0; idx < lines.length; idx++) {
    const line = lines[idx];

    // Fenced Code Blocks: ```lang
    if (line.trim().startsWith("```")) {
      if (inCodeBlock) {
        // End code block
        const highlighted = highlightCode(codeBuffer.join("\n"), codeBlockLang);
        htmlOut.push(`<div class="md-code-block">${highlighted}</div>`);
        codeBuffer = [];
        inCodeBlock = false;
      } else {
        // Start code block
        inCodeBlock = true;
        const langStr = line.trim().slice(3).trim();
        codeBlockLang = detectLanguage(`test.${langStr}`);
      }
      continue;
    }

    if (inCodeBlock) {
      codeBuffer.push(line);
      continue;
    }

    // Horizontal Rule
    if (/^(\*\*\*|---|___)$/.test(line.trim())) {
      htmlOut.push(`<hr class="md-hr" />`);
      continue;
    }

    // Headings: #, ##, ###
    if (line.startsWith("# ")) {
      htmlOut.push(`<h1 class="md-h1">${parseInlineMd(line.slice(2))}</h1>`);
      continue;
    }
    if (line.startsWith("## ")) {
      htmlOut.push(`<h2 class="md-h2">${parseInlineMd(line.slice(3))}</h2>`);
      continue;
    }
    if (line.startsWith("### ")) {
      htmlOut.push(`<h3 class="md-h3">${parseInlineMd(line.slice(4))}</h3>`);
      continue;
    }
    if (line.startsWith("#### ")) {
      htmlOut.push(`<h4 class="md-h4">${parseInlineMd(line.slice(5))}</h4>`);
      continue;
    }

    // Blockquote: >
    if (line.startsWith("> ")) {
      htmlOut.push(`<blockquote class="md-quote">${parseInlineMd(line.slice(2))}</blockquote>`);
      continue;
    }

    // Unordered List: - or *
    if (/^[\*\-]\s+/.test(line)) {
      htmlOut.push(`<li class="md-li">${parseInlineMd(line.replace(/^[\*\-]\s+/, ""))}</li>`);
      continue;
    }

    // Ordered List: 1.
    if (/^\d+\.\s+/.test(line)) {
      htmlOut.push(`<li class="md-oli">${parseInlineMd(line.replace(/^\d+\.\s+/, ""))}</li>`);
      continue;
    }

    // Blank line
    if (!line.trim()) {
      htmlOut.push(`<div class="md-spacing"></div>`);
      continue;
    }

    // Paragraph
    htmlOut.push(`<p class="md-p">${parseInlineMd(line)}</p>`);
  }

  if (inCodeBlock && codeBuffer.length > 0) {
    const highlighted = highlightCode(codeBuffer.join("\n"), codeBlockLang);
    htmlOut.push(`<div class="md-code-block">${highlighted}</div>`);
  }

  return htmlOut.join("\n");
}

function parseInlineMd(text: string): string {
  let s = escapeHtml(text);
  // Inline code: `code`
  s = s.replace(/`([^`]+)`/g, '<code class="md-inline-code font-mono">$1</code>');
  // Bold: **text**
  s = s.replace(/\*\*([^*]+)\*\*/g, '<strong class="md-bold">$1</strong>');
  // Italic: *text*
  s = s.replace(/\*([^*]+)\*/g, '<em class="md-italic">$1</em>');
  // Links: [label](url)
  s = s.replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" class="md-link" target="_blank" rel="noopener noreferrer">$1</a>');
  return s;
}

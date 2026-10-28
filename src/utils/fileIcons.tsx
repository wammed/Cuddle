import React from "react";
import {
  Folder,
  Binary,
  FileText,
  FileCode,
  FileCog,
  FileSpreadsheet,
  Database,
  Image as ImageIcon,
  Music,
  Video,
  Archive,
  Terminal,
} from "lucide-react";
import type { FileItem } from "../types/FileItem";

export interface FileIconProps {
  item: FileItem;
  size?: number;
  className?: string;
}

/**
 * Extracts lowercase file extension from filename.
 */
export function getFileExtension(filename: string): string {
  const parts = filename.split(".");
  if (parts.length > 1) {
    return parts[parts.length - 1].toLowerCase();
  }
  return "";
}

/**
 * Returns an expressive, color-coded Lucide icon based on file type and extension.
 * Phase 10 / Task 2-5: Rich Extension Icons
 */
export function FileIcon({ item, size = 16, className = "" }: FileIconProps): React.JSX.Element {
  if (item.is_dir) {
    return <Folder size={size} className={`item-icon-folder ${className}`} />;
  }

  // Non-UTF-8 path warning representation
  if (item.display_name.includes("\uFFFD")) {
    return (
      <span title="Non-UTF-8 binary path" className="inline-flex items-center">
        <Binary size={size} className={`item-icon-non-utf8 ${className}`} />
      </span>
    );
  }

  const ext = getFileExtension(item.display_name);

  // 1. Programming & Source Code
  switch (ext) {
    case "rs":
      return (
        <span title="Rust source file" className="inline-flex items-center">
          <FileCode size={size} className={`file-icon-rust ${className}`} />
        </span>
      );
    case "ts":
    case "tsx":
      return (
        <span title="TypeScript file" className="inline-flex items-center">
          <FileCode size={size} className={`file-icon-typescript ${className}`} />
        </span>
      );
    case "js":
    case "jsx":
    case "mjs":
    case "cjs":
      return (
        <span title="JavaScript file" className="inline-flex items-center">
          <FileCode size={size} className={`file-icon-javascript ${className}`} />
        </span>
      );
    case "py":
    case "ipynb":
      return (
        <span title="Python file" className="inline-flex items-center">
          <FileCode size={size} className={`file-icon-python ${className}`} />
        </span>
      );
    case "html":
    case "htm":
    case "css":
    case "scss":
    case "sass":
    case "less":
      return (
        <span title="Web file" className="inline-flex items-center">
          <FileCode size={size} className={`file-icon-web ${className}`} />
        </span>
      );
    case "c":
    case "cpp":
    case "cc":
    case "cxx":
    case "h":
    case "hpp":
      return (
        <span title="C/C++ file" className="inline-flex items-center">
          <FileCode size={size} className={`file-icon-cpp ${className}`} />
        </span>
      );

    // 2. Data & Configuration
    case "json":
    case "json5":
      return (
        <span title="JSON data" className="inline-flex items-center">
          <FileCog size={size} className={`file-icon-json ${className}`} />
        </span>
      );
    case "toml":
    case "yaml":
    case "yml":
      return (
        <span title="Configuration file" className="inline-flex items-center">
          <FileCog size={size} className={`file-icon-config ${className}`} />
        </span>
      );
    case "csv":
    case "tsv":
    case "xlsx":
    case "xls":
      return (
        <span title="Spreadsheet / Tabular data" className="inline-flex items-center">
          <FileSpreadsheet size={size} className={`file-icon-spreadsheet ${className}`} />
        </span>
      );
    case "sql":
    case "db":
    case "sqlite":
    case "sqlite3":
      return (
        <span title="Database file" className="inline-flex items-center">
          <Database size={size} className={`file-icon-database ${className}`} />
        </span>
      );

    // 3. Documents & Markup
    case "md":
    case "markdown":
      return (
        <span title="Markdown document" className="inline-flex items-center">
          <FileText size={size} className={`file-icon-markdown ${className}`} />
        </span>
      );
    case "pdf":
      return (
        <span title="PDF document" className="inline-flex items-center">
          <FileText size={size} className={`file-icon-pdf ${className}`} />
        </span>
      );
    case "txt":
    case "log":
      return (
        <span title="Text file" className="inline-flex items-center">
          <FileText size={size} className={`file-icon-text ${className}`} />
        </span>
      );

    // 4. Media (Images, Audio, Video)
    case "png":
    case "jpg":
    case "jpeg":
    case "gif":
    case "webp":
    case "svg":
    case "avif":
    case "bmp":
    case "ico":
      return (
        <span title="Image file" className="inline-flex items-center">
          <ImageIcon size={size} className={`file-icon-image ${className}`} />
        </span>
      );
    case "mp3":
    case "wav":
    case "ogg":
    case "flac":
    case "m4a":
    case "aac":
      return (
        <span title="Audio file" className="inline-flex items-center">
          <Music size={size} className={`file-icon-audio ${className}`} />
        </span>
      );
    case "mp4":
    case "mkv":
    case "webm":
    case "avi":
    case "mov":
      return (
        <span title="Video file" className="inline-flex items-center">
          <Video size={size} className={`file-icon-video ${className}`} />
        </span>
      );

    // 5. Archives
    case "tar":
    case "gz":
    case "tgz":
    case "bz2":
    case "xz":
    case "zip":
    case "zst":
    case "7z":
    case "rar":
      return (
        <span title="Compressed archive" className="inline-flex items-center">
          <Archive size={size} className={`file-icon-archive ${className}`} />
        </span>
      );

    // 6. Scripts & Binaries
    case "sh":
    case "bash":
    case "zsh":
    case "fish":
      return (
        <span title="Shell script" className="inline-flex items-center">
          <Terminal size={size} className={`file-icon-shell ${className}`} />
        </span>
      );

    default:
      return <FileText size={size} className={`item-icon-file ${className}`} />;
  }
}

import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import App from "../App";
import { changeLanguage } from "../i18n";

// Polyfill ResizeObserver for jsdom
class ResizeObserverMock {
  observe() {}
  unobserve() {}
  disconnect() {}
}
globalThis.ResizeObserver = ResizeObserverMock as unknown as typeof ResizeObserver;

// Mock Tauri API invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
    if (cmd === "greet") {
      return Promise.resolve("Hello, World! You've been greeted from Rust!");
    }
    if (cmd === "get_landlock_status") {
      return Promise.resolve({ supported: true, abi_version: "V5" });
    }
    if (cmd === "get_system_paths") {
      return Promise.resolve({
        home_dir: "/home/susie",
        home_raw_base64: "L2hvbWUvc3VzaWU=",
        current_dir: "/home/susie/git/wammed/Cuddle",
        current_raw_base64: "L2hvbWUvc3VzaWUvZ2l0L3dhbW1lZC9DdWRkbGU=",
      });
    }
    if (cmd === "resolve_path") {
      return Promise.resolve({
        path_str: "/home/susie",
        raw_base64: "L2hvbWUvc3VzaWU=",
        exists: true,
        is_dir: true,
        parent_path_str: "/home",
        parent_raw_base64: "L2hvbWU=",
      });
    }
    if (cmd === "get_demo_items") {
      return Promise.resolve([
        {
          item_id: "00000000-0000-0000-0000-000000000001",
          raw_base64: "L2hvbWUvc3VzaWUvZG9jdW1lbnRzL3NhbXBsZV9wcm9qZWN0Lm1k",
          display_name: "sample_project.md",
          parent_raw_base64: null,
          is_dir: false,
          is_symlink: false,
          size: 2048n,
          mtime_ms: 1729750000000n,
          readonly: false,
          inode: 12345678n,
          dev: 66305n,
        },
        {
          item_id: "00000000-0000-0000-0000-000000000002",
          raw_base64: "L2hvbWUvc3VzaWUvbGVnYWN5X2RhdGEv//5fbm9uX3V0ZjhfYXJjaGl2ZS50YXI=",
          display_name: "\uFFFD\uFFFD_non_utf8_archive.tar",
          parent_raw_base64: null,
          is_dir: false,
          is_symlink: false,
          size: 1048576n,
          mtime_ms: 1729750000000n,
          readonly: false,
          inode: 87654321n,
          dev: 66305n,
        },
      ]);
    }
    if (cmd === "scan_dir") {
      return Promise.resolve([
        {
          item_id: "00000000-0000-0000-0000-000000000001",
          raw_base64: "L2hvbWUvc3VzaWUvZG9jdW1lbnRzL3NhbXBsZV9wcm9qZWN0Lm1k",
          display_name: "sample_project.md",
          parent_raw_base64: null,
          is_dir: false,
          is_symlink: false,
          size: 2048n,
          mtime_ms: 1729750000000n,
          readonly: false,
          inode: 12345678n,
          dev: 66305n,
        },
      ]);
    }
    if (cmd === "get_app_config") {
      return Promise.resolve({
        app: { language: "auto", theme: "dark" },
        workspace: { temp_directory: null, min_free_space_multiplier: 1.5 },
        security: { enforce_landlock: true },
      });
    }
    if (cmd === "search_files_by_name") {
      return Promise.resolve({
        items: [],
        engine_used: "plocate",
        total_matches: 0,
      });
    }
    if (cmd === "search_fulltext") {
      return Promise.resolve([]);
    }
    if (cmd === "check_operation_policy") {
      return Promise.resolve({
        allowed: true,
        is_in_home: true,
        is_external_or_mount: false,
        can_reflect_original: true,
        conflict_detected: false,
        error_code: null,
        error_message: null,
      });
    }
    if (cmd === "create_original_snapshot") {
      return Promise.resolve({
        item_id: "00000000-0000-0000-0000-000000000001",
        raw_base64: "L2hvbWUvc3VzaWUvZG9jdW1lbnRzL3NhbXBsZV9wcm9qZWN0Lm1k",
        display_name: "sample_project.md",
        canonical_path: "/home/susie/documents/sample_project.md",
        inode: 12345678n,
        dev: 66305n,
        size: 2048n,
        mtime_ms: 1729750000000n,
        sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
      });
    }
    if (cmd === "verify_original_conflict") {
      return Promise.resolve({
        allowed: true,
        is_in_home: true,
        is_external_or_mount: false,
        can_reflect_original: true,
        conflict_detected: false,
        error_code: null,
        error_message: null,
      });
    }
    return Promise.resolve("");
  }),
}));

describe("App component (6-Region Main Shell)", () => {
  it("renders 6-region layout panes and responds to dynamic language toggle", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    // In Japanese
    expect(screen.getByRole("heading", { name: "Cuddle" })).toBeDefined();
    expect(screen.getByText("検索結果")).toBeDefined();
    expect(screen.getByText("ディレクトリツリー")).toBeDefined();
    expect(screen.getByText("ファイルブラウザー")).toBeDefined();
    expect(screen.getAllByText("作業ワークスペース").length).toBeGreaterThan(0);
    expect(screen.getAllByText("アイテム詳細").length).toBeGreaterThan(0);
    expect(screen.getByTitle("言語切替")).toBeDefined();

    // Toggle to English
    const toggleBtn = screen.getByTitle("言語切替");
    await act(async () => {
      fireEvent.click(toggleBtn);
    });

    // In English
    expect(screen.getByRole("heading", { name: "Cuddle" })).toBeDefined();
    expect(screen.getByText("Search Results")).toBeDefined();
    expect(screen.getByText("Directory Tree")).toBeDefined();
    expect(screen.getByText("File Browser")).toBeDefined();
    expect(screen.getAllByText("Workbench").length).toBeGreaterThan(0);
    expect(screen.getAllByText("Item Details").length).toBeGreaterThan(0);
    expect(screen.getByTitle("Language")).toBeDefined();
  });

  it("supports sidebar and bottom panel visibility toggling", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    // Toggle sidebar off
    const sidebarToggleBtn = screen.getByTitle("サイドバー切替");
    await act(async () => {
      fireEvent.click(sidebarToggleBtn);
    });

    // The heading inside the sidebar panel should be unmounted
    expect(screen.queryByRole("heading", { name: "アイテム詳細" })).toBeNull();

    // Toggle sidebar back on
    await act(async () => {
      fireEvent.click(sidebarToggleBtn);
    });
    expect(screen.getByRole("heading", { name: "アイテム詳細" })).toBeDefined();
  });

  it("focuses search input on Ctrl+F keyboard shortcut (Phase 8.4)", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    const searchInput = document.getElementById("global-search-input") as HTMLInputElement;
    expect(searchInput).not.toBeNull();

    // Fire Ctrl+F
    fireEvent.keyDown(window, { key: "f", ctrlKey: true });
    expect(document.activeElement).toBe(searchInput);
  });

  it("opens settings modal when clicking settings button in header", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    const settingsBtn = screen.getByTitle("環境設定 & 外観");
    expect(settingsBtn).toBeDefined();

    await act(async () => {
      fireEvent.click(settingsBtn);
    });

    expect(screen.getByText("環境設定 & 外観")).toBeDefined();
    expect(screen.getByText("外観 & テーマ (30種)")).toBeDefined();
  });

  it("toggles between workbench and start page", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    const startTabBtn = screen.getByTitle("Start Dashboard");
    await act(async () => {
      fireEvent.click(startTabBtn);
    });

    expect(screen.getByText("安全第一の原本保護型ファイル作業環境")).toBeDefined();
    expect(screen.getByText("主要機能・ワークフロー")).toBeDefined();
  });
});

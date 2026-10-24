import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, act } from "@testing-library/react";
import App from "../App";
import "../i18n";
import { changeLanguage } from "../i18n";

// Mock Tauri API invoke
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string, args: { name?: string }) => {
    if (cmd === "greet") {
      return Promise.resolve(`Hello, ${args.name || "World"}! You've been greeted from Rust!`);
    }
    if (cmd === "get_landlock_status") {
      return Promise.resolve({ supported: true, abi_version: "V5" });
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
        },
      ]);
    }
    return Promise.resolve("");
  }),
}));

describe("App component", () => {
  it("renders brand title, dual-key non-UTF-8 items, and responds to dynamic language toggle", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    // In Japanese
    expect(screen.getByText("FileWorkbench (Cuddle)")).toBeDefined();
    expect(screen.getByText("二重キーファイルIPC（UTF-8 & 非UTF-8）")).toBeDefined();
    expect(screen.getByTitle("言語切替")).toBeDefined();

    // Toggle to English
    const toggleBtn = screen.getByTitle("言語切替");
    await act(async () => {
      fireEvent.click(toggleBtn);
    });

    // In English
    expect(screen.getByText("FileWorkbench (Cuddle)")).toBeDefined();
    expect(screen.getByText("Dual-Key File IPC (UTF-8 & Non-UTF-8)")).toBeDefined();
    expect(screen.getByTitle("Language")).toBeDefined();
  });
});


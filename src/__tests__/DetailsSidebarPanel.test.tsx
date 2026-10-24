import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent, waitFor, act } from "@testing-library/react";
import { DetailsSidebarPanel } from "../components/DetailsSidebarPanel";
import { changeLanguage } from "../i18n";
import type { FileItem } from "../types/FileItem";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockImplementation((cmd: string) => {
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
        item_id: "test-item-1",
        raw_base64: "L2hvbWUvc3VzaWUvdGVzdC50eHQ=",
        display_name: "test.txt",
        canonical_path: "/home/susie/test.txt",
        inode: 123456n,
        dev: 66305n,
        size: 1024n,
        mtime_ms: 1729750000000n,
        sha256: "9f86d081884c7d659a2feaa0c55ad015a3bf4f1b2b0b822cd15d6c15b0f00a08",
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

describe("DetailsSidebarPanel & OperationPolicy Integration", () => {
  const sampleItem: FileItem = {
    item_id: "test-item-1",
    raw_base64: "L2hvbWUvc3VzaWUvdGVzdC50eHQ=",
    display_name: "test.txt",
    parent_raw_base64: null,
    is_dir: false,
    is_symlink: false,
    size: 1024n,
    mtime_ms: 1729750000000n,
    readonly: false,
    inode: 123456n,
    dev: 66305n,
  };

  it("evaluates policy and displays safe-to-reflect status in Japanese", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(
      <DetailsSidebarPanel
        item={sampleItem}
        onClose={vi.fn()}
        onExtractText={vi.fn()}
        extractLoading={false}
        extractedInfo={null}
      />
    );

    // Initial render shows item name
    expect(screen.getByText("test.txt")).toBeDefined();

    // After policy evaluation resolves
    await waitFor(() => {
      expect(screen.getByText("原本への反映可能")).toBeDefined();
    });

    // Button to capture snapshot
    const snapBtn = screen.getByText(/原本スナップショット記録済み/);
    expect(snapBtn).toBeDefined();

    await act(async () => {
      fireEvent.click(snapBtn);
    });

    // After snapshot creation, SHA-256 hash preview is displayed
    await waitFor(() => {
      expect(screen.getByText(/SHA256: 9f86d081884c7d65/)).toBeDefined();
    });
  });

  it("displays policy and snapshot status in English", async () => {
    await act(async () => {
      await changeLanguage("en");
    });

    render(
      <DetailsSidebarPanel
        item={sampleItem}
        onClose={vi.fn()}
        onExtractText={vi.fn()}
        extractLoading={false}
        extractedInfo={null}
      />
    );

    await waitFor(() => {
      expect(screen.getByText("Eligible for reflecting back to original")).toBeDefined();
    });
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act, waitFor } from "@testing-library/react";
import { SearchPanel } from "../components/SearchPanel";
import { changeLanguage } from "../i18n";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string, args?: any) => {
    if (cmd === "get_index_stats") {
      return {
        total_documents: 42,
        index_path: "/home/susie/.local/share/Cuddle/index",
      };
    }
    if (cmd === "run_index_operation") {
      return {
        mode: args?.options?.mode || "IncrementalUpdate",
        target_path: "/test/path",
        indexed_count: 5,
        updated_count: 2,
        unchanged_count: 10,
        skipped_count: 1,
        failed_count: 0,
        total_documents: 47,
        elapsed_ms: 120,
        message: "Index operation completed: 5 indexed, 2 updated",
      };
    }
    if (cmd === "search_files_by_name") {
      return {
        items: [
          {
            item_id: "hit-1",
            display_name: "test.txt",
            raw_base64: "dGVzdC50eHQ=",
            size: 100,
            inode: 111,
          },
        ],
        engine_used: "plocate",
        total_matches: 1,
      };
    }
    if (cmd === "search_fulltext") {
      return [
        {
          item_id: "hit-2",
          display_name: "guide.md",
          raw_base64: "Z3VpZGUubWQ=",
          snippet: "This is a <b>match</b> snippet.",
          score: 1.5,
          size: 200,
          mtime_ms: 1729750000000,
        },
      ];
    }
    return null;
  }),
}));

describe("SearchPanel with Explicit Indexing Dropdown", () => {
  beforeEach(async () => {
    await act(async () => {
      await changeLanguage("ja");
    });
  });

  it("renders search input, search button, and index management switch button", async () => {
    const handleSelect = vi.fn();
    const handleLog = vi.fn();

    await act(async () => {
      render(
        <SearchPanel
          onSelectItem={handleSelect}
          onLog={handleLog}
          currentPath="/home/susie/documents"
        />
      );
    });

    expect(screen.getByPlaceholderText(/ファイル名を入力/)).toBeDefined();
    expect(screen.getByRole("button", { name: "検索" })).toBeDefined();
    expect(
      screen.getByRole("button", { name: /インデックス/ })
    ).toBeDefined();
  });

  it("opens dropdown menu with 4 operation modes when clicking the index switch", async () => {
    const handleSelect = vi.fn();
    const handleLog = vi.fn();

    await act(async () => {
      render(
        <SearchPanel
          onSelectItem={handleSelect}
          onLog={handleLog}
          currentPath="/home/susie/documents"
        />
      );
    });

    const switchBtn = screen.getByRole("button", { name: /インデックス/ });
    await act(async () => {
      fireEvent.click(switchBtn);
    });

    expect(screen.getByText("インデックス操作")).toBeDefined();
    expect(screen.getByText("差分更新")).toBeDefined();
    expect(screen.getByText("フォルダをインデックス化")).toBeDefined();
    expect(screen.getByText("完全再構築")).toBeDefined();
    expect(screen.getByText("インデックス初期化")).toBeDefined();
  });

  it("triggers run_index_operation when an operation is selected", async () => {
    const handleSelect = vi.fn();
    const handleLog = vi.fn();

    await act(async () => {
      render(
        <SearchPanel
          onSelectItem={handleSelect}
          onLog={handleLog}
          currentPath="/home/susie/documents"
        />
      );
    });

    const switchBtn = screen.getByRole("button", { name: /インデックス/ });
    await act(async () => {
      fireEvent.click(switchBtn);
    });

    const incBtn = screen.getByText("差分更新");
    await act(async () => {
      fireEvent.click(incBtn);
    });

    await waitFor(() => {
      expect(handleLog).toHaveBeenCalledWith(
        expect.stringContaining("IncrementalUpdate")
      );
    });

    // Check completion summary banner
    expect(
      await screen.findByText(/完了: 追加 5件 \/ 更新 2件/)
    ).toBeDefined();
  });

  it("switches to fulltext search tab and executes query", async () => {
    const handleSelect = vi.fn();
    const handleLog = vi.fn();

    await act(async () => {
      render(
        <SearchPanel
          onSelectItem={handleSelect}
          onLog={handleLog}
          currentPath="/home/susie/documents"
        />
      );
    });

    const fulltextTab = screen.getByTitle(/日本語全文検索/);
    await act(async () => {
      fireEvent.click(fulltextTab);
    });

    const input = screen.getByPlaceholderText(/本文のキーワードを入力/);
    await act(async () => {
      fireEvent.change(input, { target: { value: "Landlock" } });
    });

    const submitBtn = screen.getByRole("button", { name: "検索" });
    await act(async () => {
      fireEvent.click(submitBtn);
    });

    await waitFor(() => {
      expect(screen.getByText("guide.md")).toBeDefined();
    });
  });
});

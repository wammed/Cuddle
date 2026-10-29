import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, act, waitFor } from "@testing-library/react";
import { SearchPanel } from "../components/SearchPanel";
import { changeLanguage } from "../i18n";
import type { IndexRepairResultDto } from "../types/IndexRepairResultDto";

const mockInvoke = vi.fn();

vi.mock("@tauri-apps/api/core", () => ({
  invoke: (...args: unknown[]) => mockInvoke(...args),
}));

describe("Phase 13: Search, Indexing, and Text Extraction Evolution", () => {
  beforeEach(async () => {
    vi.clearAllMocks();
    await act(async () => {
      await changeLanguage("ja");
    });

    mockInvoke.mockImplementation(async (cmd: string, args?: any) => {
      if (cmd === "get_index_stats") {
        return {
          total_documents: 150,
          index_path: "/home/susie/.local/share/Cuddle/index",
        };
      }
      if (cmd === "get_index_repair_queue_count") {
        return 3;
      }
      if (cmd === "process_index_repair_queue") {
        const result: IndexRepairResultDto = {
          processed_count: 3,
          succeeded_count: 2,
          failed_count: 1,
          remaining_count: 0,
        };
        return result;
      }
      if (cmd === "run_index_operation") {
        return {
          mode: args?.options?.mode || "IncrementalUpdate",
          target_path: "/test/docs",
          indexed_count: 10,
          updated_count: 2,
          unchanged_count: 50,
          skipped_count: 0,
          failed_count: 0,
          total_documents: 162,
          elapsed_ms: 250,
          message: "Indexing done",
        };
      }
      if (cmd === "search_fulltext") {
        return [
          {
            item_id: "doc-1",
            display_name: "仕様書.docx",
            raw_base64: "5LuV5qeY5pu4LmRvY3g=",
            snippet: "原本保護機能と<b>Office OpenXML</b>の抽出",
            score: 4.25,
            size: 45000,
            mtime_ms: 1729800000000,
          },
          {
            item_id: "doc-2",
            display_name: "readme.html",
            raw_base64: "cmVhZG1lLmh0bWw=",
            snippet: "HTMLタグを除去した<b>プレーンテキスト</b>",
            score: 2.10,
            size: 12000,
            mtime_ms: 1729801000000,
          },
        ];
      }
      return null;
    });
  });

  describe("Task 1-1: Index Repair Queue Integration", () => {
    it("fetches and displays repair queue pending count badge in index menu", async () => {
      const handleSelect = vi.fn();
      const handleLog = vi.fn();

      await act(async () => {
        render(
          <SearchPanel
            onSelectItem={handleSelect}
            onLog={handleLog}
            currentPath="/test/docs"
          />
        );
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("get_index_repair_queue_count");
      });

      // Open index dropdown
      const switchBtn = screen.getByRole("button", { name: /インデックス/ });
      await act(async () => {
        fireEvent.click(switchBtn);
      });

      // Check repair queue item and count badge
      expect(screen.getByText("インデックス修復キューの処理")).toBeDefined();
      expect(screen.getByText("3")).toBeDefined();
    });

    it("executes process_index_repair_queue when clicked and displays bilingual result banner", async () => {
      const handleSelect = vi.fn();
      const handleLog = vi.fn();

      await act(async () => {
        render(
          <SearchPanel
            onSelectItem={handleSelect}
            onLog={handleLog}
            currentPath="/test/docs"
          />
        );
      });

      // Open index dropdown
      const switchBtn = screen.getByRole("button", { name: /インデックス/ });
      await act(async () => {
        fireEvent.click(switchBtn);
      });

      // Click repair queue button
      const repairBtn = screen.getByText("インデックス修復キューの処理");
      await act(async () => {
        fireEvent.click(repairBtn);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("process_index_repair_queue");
      });

      // Verify completion message banner
      expect(
        await screen.findByText(/修復完了: 2件成功 \/ 1件失敗 \(残り 0件\)/)
      ).toBeDefined();

      expect(handleLog).toHaveBeenCalledWith(
        expect.stringContaining("Repair queue processed")
      );
    });

    it("renders repair queue English translations when language is toggled", async () => {
      await act(async () => {
        await changeLanguage("en");
      });

      const handleSelect = vi.fn();
      const handleLog = vi.fn();

      await act(async () => {
        render(
          <SearchPanel
            onSelectItem={handleSelect}
            onLog={handleLog}
            currentPath="/test/docs"
          />
        );
      });

      const switchBtn = screen.getByRole("button", { name: /Index/ });
      await act(async () => {
        fireEvent.click(switchBtn);
      });

      expect(screen.getByText("Process Index Repair Queue")).toBeDefined();
      expect(screen.getByText("3")).toBeDefined();
    });
  });

  describe("Task 1-2 & 1-3 & 1-4: Hybrid Search & Rich Document Previews", () => {
    it("searches and renders Office OpenXML (.docx) and HTML extracted snippets", async () => {
      const handleSelect = vi.fn();
      const handleLog = vi.fn();

      await act(async () => {
        render(
          <SearchPanel
            onSelectItem={handleSelect}
            onLog={handleLog}
            currentPath="/test/docs"
          />
        );
      });

      // Switch to Full-text tab
      const fulltextTab = screen.getByTitle(/全文検索/);
      await act(async () => {
        fireEvent.click(fulltextTab);
      });

      // Search for query
      const input = screen.getByPlaceholderText(/本文/);
      await act(async () => {
        fireEvent.change(input, { target: { value: "Office OpenXML" } });
      });

      const submitBtn = screen.getByRole("button", { name: "検索" });
      await act(async () => {
        fireEvent.click(submitBtn);
      });

      await waitFor(() => {
        expect(mockInvoke).toHaveBeenCalledWith("search_fulltext", {
          query: "Office OpenXML",
          options: null,
        });
      });

      // Check results
      expect(await screen.findByText("仕様書.docx")).toBeDefined();
      expect(screen.getByText("readme.html")).toBeDefined();
      expect(screen.getByText(/原本保護機能と/)).toBeDefined();
      expect(screen.getByText(/HTMLタグを除去した/)).toBeDefined();
    });

    it("passes include_metadata_only_excluded flag in run_index_operation", async () => {
      const handleSelect = vi.fn();
      const handleLog = vi.fn();

      await act(async () => {
        render(
          <SearchPanel
            onSelectItem={handleSelect}
            onLog={handleLog}
            currentPath="/test/docs"
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
        expect(mockInvoke).toHaveBeenCalledWith("run_index_operation", {
          options: expect.objectContaining({
            mode: "IncrementalUpdate",
            include_metadata_only_excluded: false,
          }),
        });
      });
    });
  });
});

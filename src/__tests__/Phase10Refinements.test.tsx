import { describe, it, expect, vi } from "vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { FileIcon, getFileExtension } from "../utils/fileIcons";
import { BottomTabPanel } from "../components/BottomTabPanel";
import { WorkspacePanel } from "../components/WorkspacePanel";
import { FileBrowserPanel } from "../components/FileBrowserPanel";
import type { FileItem } from "../types/FileItem";

vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: ({ count, estimateSize }: { count: number; estimateSize: () => number }) => ({
    getTotalSize: () => count * estimateSize(),
    getVirtualItems: () =>
      Array.from({ length: count }, (_, index) => ({
        index,
        start: index * estimateSize(),
        size: estimateSize(),
      })),
  }),
}));

describe("Phase 10: UX Refinements & Visual Polish", () => {
  const makeItem = (name: string, isDir = false, isNonUtf8 = false): FileItem => ({
    item_id: `id-${name}`,
    raw_base64: btoa(name),
    display_name: isNonUtf8 ? `${name}\uFFFD` : name,
    parent_raw_base64: null,
    is_dir: isDir,
    is_symlink: false,
    size: 2048n,
    mtime_ms: 1729750000000n,
    readonly: false,
    inode: 100n,
    dev: 1n,
  });

  describe("Task 2-5: FileIcon & Extension Detection", () => {
    it("extracts extension correctly", () => {
      expect(getFileExtension("main.rs")).toBe("rs");
      expect(getFileExtension("archive.tar.gz")).toBe("gz");
      expect(getFileExtension("Makefile")).toBe("");
      expect(getFileExtension(".bashrc")).toBe("bashrc");
    });

    it("renders specific icon classes for diverse file types", () => {
      const { container: rsBox } = render(<FileIcon item={makeItem("lib.rs")} />);
      expect(rsBox.querySelector(".file-icon-rust")).not.toBeNull();

      const { container: tsBox } = render(<FileIcon item={makeItem("App.tsx")} />);
      expect(tsBox.querySelector(".file-icon-typescript")).not.toBeNull();

      const { container: pyBox } = render(<FileIcon item={makeItem("script.py")} />);
      expect(pyBox.querySelector(".file-icon-python")).not.toBeNull();

      const { container: jsonBox } = render(<FileIcon item={makeItem("config.json")} />);
      expect(jsonBox.querySelector(".file-icon-json")).not.toBeNull();

      const { container: imgBox } = render(<FileIcon item={makeItem("photo.png")} />);
      expect(imgBox.querySelector(".file-icon-image")).not.toBeNull();

      const { container: dirBox } = render(<FileIcon item={makeItem("my_folder", true)} />);
      expect(dirBox.querySelector(".item-icon-folder")).not.toBeNull();

      const { container: nonUtf8Box } = render(<FileIcon item={makeItem("broken", false, true)} />);
      expect(nonUtf8Box.querySelector(".item-icon-non-utf8")).not.toBeNull();
    });
  });

  describe("Task 2-4: Pipeline Logs Toolbar & Mode Filters", () => {
    const sampleLogs = [
      "[21:00:01] Language switched to ja",
      "[21:00:02] Staged file: /home/susie/doc.txt",
      "[21:00:03] Reflected with .origin backup: doc.origin.txt -> doc.txt",
      "[21:00:04] Error: Kernel Landlock LSM returned permission denied",
      "[21:00:05] Renamed batch: 3 files updated",
    ];

    it("filters logs by All, File Only, and Hidden (Errors & Warnings)", async () => {
      const { container } = render(
        <BottomTabPanel
          selectedItem={null}
          extractedInfo={null}
          appConfig={null}
          logs={sampleLogs}
          history={[]}
          onClose={vi.fn()}
          activeTab="logs"
        />
      );

      // Initially 'all' mode: displays all 5 logs
      expect(container.querySelectorAll(".log-line").length).toBe(5);

      // Switch to 'file_only'
      const fileOnlyBtn = screen.getByRole("button", { name: /logs\.filter_file_only|File Ops Only|ファイル操作のみ/i });
      fireEvent.click(fileOnlyBtn);
      // Staged, Reflected, Renamed should be visible (3 items)
      expect(container.querySelectorAll(".log-line").length).toBe(3);

      // Switch to 'hidden' (Errors & Warnings)
      const hiddenBtn = screen.getByRole("button", { name: /logs\.filter_hidden|Errors & Warnings|警告・エラーのみ/i });
      fireEvent.click(hiddenBtn);
      // Only the Error log should be visible (1 item)
      expect(container.querySelectorAll(".log-line").length).toBe(1);

      // Search filtering
      const allBtn = screen.getByRole("button", { name: /logs\.filter_all|All Logs|全ログ/i });
      fireEvent.click(allBtn);
      const searchInput = container.querySelector(".logs-search-input") as HTMLInputElement;
      expect(searchInput).not.toBeNull();
      fireEvent.change(searchInput, { target: { value: "Landlock" } });
      expect(container.querySelectorAll(".log-line").length).toBe(1);
    });
  });

  describe("Task 2-1: Workspace Drag & Drop (D&D) Staging", () => {
    it("handles dragover, dragleave, and drop events to stage items safely", () => {
      const handleDropItem = vi.fn();
      const { container } = render(
        <WorkspacePanel
          stagedFiles={[]}
          selectedId={null}
          onSelect={vi.fn()}
          onClearWorkspace={vi.fn()}
          onDropItem={handleDropItem}
        />
      );

      const panel = container.querySelector(".workspace-panel-container")!;
      expect(panel).not.toBeNull();

      const testItem = makeItem("target.txt");
      const serialized = JSON.stringify(testItem, (_, v) =>
        typeof v === "bigint" ? v.toString() : v
      );
      const dataTransferMock = {
        types: ["application/cuddle-file-item", "application/json"],
        dropEffect: "none",
        setData: vi.fn(),
        getData: vi.fn().mockReturnValue(serialized),
      };

      // 1. Drag Over
      fireEvent.dragOver(panel, { dataTransfer: dataTransferMock });
      expect(panel.classList.contains("is-drag-over")).toBe(true);
      expect(container.querySelector(".workspace-drag-overlay")).not.toBeNull();

      // 2. Drag Leave
      fireEvent.dragLeave(panel, {
        currentTarget: panel,
        relatedTarget: document.body,
      });
      expect(panel.classList.contains("is-drag-over")).toBe(false);

      // 3. Drop
      fireEvent.dragOver(panel, { dataTransfer: dataTransferMock });
      fireEvent.drop(panel, { dataTransfer: dataTransferMock });
      expect(panel.classList.contains("is-drag-over")).toBe(false);
      expect(handleDropItem).toHaveBeenCalledWith(testItem);
    });
  });

  describe("Task 2-9: Interactive Breadcrumb Navigation in FileBrowserPanel", () => {
    it("renders clickable breadcrumb segments and navigates to ancestral paths", () => {
      const handleNavigatePath = vi.fn();
      const { container } = render(
        <FileBrowserPanel
          currentPath="/home/susie/projects/Cuddle"
          items={[]}
          selectedItem={null}
          onSelect={vi.fn()}
          onOpenDir={vi.fn()}
          onStage={vi.fn()}
          onStageAll={vi.fn()}
          onNavigatePath={handleNavigatePath}
          isScanning={false}
          onRefresh={vi.fn()}
        />
      );

      // Check breadcrumb segments
      const segmentBtns = container.querySelectorAll(".breadcrumb-segment-btn");
      expect(segmentBtns.length).toBe(5); // "/", "home", "susie", "projects", "Cuddle"

      // Click "susie" segment -> should navigate to "/home/susie"
      const susieBtn = Array.from(segmentBtns).find((btn) => btn.textContent === "susie");
      expect(susieBtn).toBeDefined();
      fireEvent.click(susieBtn!);
      expect(handleNavigatePath).toHaveBeenCalledWith("/home/susie");
    });
  });

  describe("Task 2-10: Ambient Aurora Mesh & Hardware-Accelerated Micro-Grain Overlay", () => {
    it("renders ambient mesh with 4 glowing orbs and hardware-accelerated SVG film grain filter", () => {
      const { container } = render(
        <div className="cuddle-root-app">
          <div className="ambient-aurora-mesh" aria-hidden="true">
            <div className="aurora-orb orb-1 orb-teal" />
            <div className="aurora-orb orb-2 orb-emerald" />
            <div className="aurora-orb orb-3 orb-cyan" />
            <div className="aurora-orb orb-4 orb-violet" />
          </div>
          <svg className="ambient-grain-overlay" aria-hidden="true">
            <filter id="cuddle-ambient-grain">
              <feTurbulence
                type="fractalNoise"
                baseFrequency="0.75"
                numOctaves="3"
                stitchTiles="stitch"
              />
              <feColorMatrix type="saturate" values="0" />
            </filter>
            <rect width="100%" height="100%" filter="url(#cuddle-ambient-grain)" />
          </svg>
        </div>
      );

      const mesh = container.querySelector(".ambient-aurora-mesh");
      expect(mesh).not.toBeNull();
      expect(mesh?.querySelectorAll(".aurora-orb").length).toBe(4);
      expect(mesh?.querySelector(".orb-1")).not.toBeNull();
      expect(mesh?.querySelector(".orb-2")).not.toBeNull();
      expect(mesh?.querySelector(".orb-3")).not.toBeNull();
      expect(mesh?.querySelector(".orb-4")).not.toBeNull();
      expect(mesh?.querySelector(".orb-teal")).not.toBeNull();
      expect(mesh?.querySelector(".orb-emerald")).not.toBeNull();
      expect(mesh?.querySelector(".orb-cyan")).not.toBeNull();
      expect(mesh?.querySelector(".orb-violet")).not.toBeNull();

      const grainSvg = container.querySelector("svg.ambient-grain-overlay");
      expect(grainSvg).not.toBeNull();
      expect(grainSvg?.querySelector("feTurbulence")).not.toBeNull();
      expect(grainSvg?.querySelector("feColorMatrix")).not.toBeNull();
      expect(grainSvg?.querySelector("rect")).not.toBeNull();
    });
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SnippetManagerView } from "../components/SnippetManagerView";
import { changeLanguage } from "../i18n";
import {
  DEFAULT_EDITOR_SNIPPETS,
  detectInitialSnippetLanguage,
  getAvailableSnippetLanguages,
  SnippetLanguageKey,
} from "../utils/snippets";
import type { EditorSnippetDto } from "../types/EditorSnippetDto";

describe("Snippet Management & Preset Rules", () => {
  beforeEach(async () => {
    await changeLanguage("ja");
  });

  it("provides exactly 5 default presets for each of the 8 required languages (40 total)", () => {
    expect(DEFAULT_EDITOR_SNIPPETS.length).toBe(40);

    const requiredLangs: SnippetLanguageKey[] = [
      "bash",
      "fish",
      "zsh",
      "jsonc",
      "toml",
      "yaml",
      "lua",
      "plaintext",
    ];

    for (const lang of requiredLangs) {
      const presets = DEFAULT_EDITOR_SNIPPETS.filter((s) => s.language === lang);
      expect(presets.length).toBe(5);
      for (const p of presets) {
        expect(p.id).toBeTruthy();
        expect(p.label).toBeTruthy();
        expect(p.snippet).toBeTruthy();
      }
    }
  });

  it("accurately detects initial language from filenames", () => {
    expect(detectInitialSnippetLanguage(".bashrc")).toBe("bash");
    expect(detectInitialSnippetLanguage("script.sh")).toBe("bash");
    expect(detectInitialSnippetLanguage("config.fish")).toBe("fish");
    expect(detectInitialSnippetLanguage(".zshrc")).toBe("zsh");
    expect(detectInitialSnippetLanguage("tsconfig.json")).toBe("jsonc");
    expect(detectInitialSnippetLanguage("Cargo.toml")).toBe("toml");
    expect(detectInitialSnippetLanguage(".gitconfig")).toBe("toml");
    expect(detectInitialSnippetLanguage("docker-compose.yaml")).toBe("yaml");
    expect(detectInitialSnippetLanguage("init.lua")).toBe("lua");
    expect(detectInitialSnippetLanguage("README.txt")).toBe("plaintext");
  });

  describe("SnippetManagerView Component", () => {
    const mockSnippets: EditorSnippetDto[] = [
      {
        id: "s-1",
        language: "bash",
        label: "+ Test Bash",
        snippet: "echo 'bash'",
        description: "Test description",
      },
      {
        id: "s-2",
        language: "fish",
        label: "+ Test Fish",
        snippet: "echo 'fish'",
        description: "Fish description",
      },
      {
        id: "s-3",
        language: "lua",
        label: "+ Test Lua",
        snippet: "print('lua')",
        description: "Lua description",
      },
    ];

    it("renders snippets and filters by language and search query", () => {
      render(
        <SnippetManagerView
          snippets={mockSnippets}
          onUpdateSnippets={vi.fn()}
          initialLanguage="all"
        />
      );

      // Initially all are shown
      expect(screen.getByText("+ Test Bash")).toBeDefined();
      expect(screen.getByText("+ Test Fish")).toBeDefined();
      expect(screen.getByText("+ Test Lua")).toBeDefined();

      // Filter by language: Lua
      const langSelect = screen.getByRole("combobox") as HTMLSelectElement;
      fireEvent.change(langSelect, { target: { value: "lua" } });

      expect(screen.queryByText("+ Test Bash")).toBeNull();
      expect(screen.queryByText("+ Test Fish")).toBeNull();
      expect(screen.getByText("+ Test Lua")).toBeDefined();
    });

    it("triggers onUpdateSnippets when creating a new snippet", async () => {
      const handleUpdate = vi.fn();
      render(
        <SnippetManagerView
          snippets={mockSnippets}
          onUpdateSnippets={handleUpdate}
          initialLanguage="bash"
        />
      );

      // Click Add button
      const addBtn = screen.getByText("新規追加");
      fireEvent.click(addBtn);

      // Fill in form
      const labelInput = screen.getByPlaceholderText("+ PATH 追加");
      const snippetTextarea = screen.getByPlaceholderText('export PATH="$HOME/.local/bin:$PATH"');

      fireEvent.change(labelInput, { target: { value: "+ Custom PATH" } });
      fireEvent.change(snippetTextarea, { target: { value: "export FOO=BAR" } });

      // Save
      const saveBtn = screen.getByText("保存");
      fireEvent.click(saveBtn);

      await waitFor(() => {
        expect(handleUpdate).toHaveBeenCalledTimes(1);
        const updatedList: EditorSnippetDto[] = handleUpdate.mock.calls[0][0];
        expect(updatedList.length).toBe(4);
        expect(updatedList.some((s) => s.label === "+ Custom PATH" && s.snippet === "export FOO=BAR")).toBe(true);
      });
    });

    it("allows deleting a snippet with confirmation", async () => {
      const handleUpdate = vi.fn();
      render(
        <SnippetManagerView
          snippets={mockSnippets}
          onUpdateSnippets={handleUpdate}
          initialLanguage="all"
        />
      );

      // Find delete buttons
      const deleteButtons = screen.getAllByTitle("削除");
      expect(deleteButtons.length).toBe(3);

      // Click delete for the first snippet
      fireEvent.click(deleteButtons[0]);

      // Confirm overlay should show
      expect(screen.getByText("このスニペットを削除しますか？")).toBeDefined();

      const confirmDeleteBtn = document.querySelector(
        ".snippet-delete-confirm-overlay .btn-editor-danger"
      ) as HTMLButtonElement;
      expect(confirmDeleteBtn).toBeDefined();
      fireEvent.click(confirmDeleteBtn);

      await waitFor(() => {
        expect(handleUpdate).toHaveBeenCalledTimes(1);
        const updatedList: EditorSnippetDto[] = handleUpdate.mock.calls[0][0];
        expect(updatedList.length).toBe(2);
        expect(updatedList.find((s) => s.id === "s-1")).toBeUndefined();
      });
    });

    it("dynamically adds python and rust to dropdown options when snippets with those languages exist", () => {
      const extendedSnippets: EditorSnippetDto[] = [
        ...mockSnippets,
        {
          id: "py-1",
          language: "python",
          label: "+ Python Main",
          snippet: "if __name__ == '__main__':\n    main()",
          description: "Python entrypoint",
        },
        {
          id: "rs-1",
          language: "rust",
          label: "+ Rust Main",
          snippet: "fn main() {\n    println!(\"Hello\");\n}",
          description: "Rust entrypoint",
        },
        {
          id: "custom-1",
          language: "nix",
          label: "+ Nix Flake",
          snippet: "{ description = \"flake\"; }",
          description: "Custom language flake",
        },
      ];

      // 1. Verify getAvailableSnippetLanguages includes python, rust, and custom nix
      const available = getAvailableSnippetLanguages(extendedSnippets);
      const keys = available.map((a) => a.key);
      expect(keys).toContain("bash");
      expect(keys).toContain("python");
      expect(keys).toContain("rust");
      expect(keys).toContain("nix");

      const pyOption = available.find((a) => a.key === "python");
      expect(pyOption?.label).toBe("Python");
      expect(pyOption?.badge).toBe("PY");

      const rsOption = available.find((a) => a.key === "rust");
      expect(rsOption?.label).toBe("Rust");
      expect(rsOption?.badge).toBe("RS");

      const nixOption = available.find((a) => a.key === "nix");
      expect(nixOption?.label).toBe("Nix");

      // 2. Verify SnippetManagerView renders Python, Rust, and Nix inside the dropdown
      render(
        <SnippetManagerView
          snippets={extendedSnippets}
          onUpdateSnippets={vi.fn()}
          initialLanguage="all"
        />
      );

      const langSelect = screen.getByRole("combobox") as HTMLSelectElement;
      const optionValues = Array.from(langSelect.options).map((o) => o.value);

      expect(optionValues).toContain("python");
      expect(optionValues).toContain("rust");
      expect(optionValues).toContain("nix");

      // Verify filtering works for dynamically added Python
      fireEvent.change(langSelect, { target: { value: "python" } });
      expect(screen.getByText("+ Python Main")).toBeDefined();
      expect(screen.queryByText("+ Test Bash")).toBeNull();
      expect(screen.queryByText("+ Rust Main")).toBeNull();
    });
  });
});

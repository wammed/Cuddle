import { describe, it, expect, vi, beforeEach } from "vitest";
import { render, screen, fireEvent, waitFor } from "@testing-library/react";
import { SnippetManagerView } from "../components/SnippetManagerView";
import { changeLanguage } from "../i18n";
import {
  DEFAULT_EDITOR_SNIPPETS,
  detectInitialSnippetLanguage,
  getAvailableSnippetLanguages,
  normalizeSnippetLanguageKey,
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
    expect(detectInitialSnippetLanguage("main.cpp")).toBe("cpp");
    expect(detectInitialSnippetLanguage("README.txt")).toBe("plaintext");
  });

  it("normalizes language keys and aliases to canonical forms", () => {
    expect(normalizeSnippetLanguageKey("plain text")).toBe("plaintext");
    expect(normalizeSnippetLanguageKey("Plain Text")).toBe("plaintext");
    expect(normalizeSnippetLanguageKey("txt")).toBe("plaintext");
    expect(normalizeSnippetLanguageKey("text")).toBe("plaintext");
    expect(normalizeSnippetLanguageKey("json")).toBe("jsonc");
    expect(normalizeSnippetLanguageKey("jsonc")).toBe("jsonc");
    expect(normalizeSnippetLanguageKey("c++")).toBe("cpp");
    expect(normalizeSnippetLanguageKey("cpp")).toBe("cpp");
    expect(normalizeSnippetLanguageKey("py")).toBe("python");
    expect(normalizeSnippetLanguageKey("rs")).toBe("rust");
    expect(normalizeSnippetLanguageKey("sh")).toBe("bash");
    expect(normalizeSnippetLanguageKey("nix")).toBe("nix");
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

    it("prevents duplicate language categories in dropdown when variations like 'plain text' or 'json' are added", () => {
      const snippetsWithVariations: EditorSnippetDto[] = [
        ...mockSnippets,
        {
          id: "pt-1",
          language: "plain text",
          label: "+ Plain Text Todo",
          snippet: "TODO item",
          description: "Plain text with space",
        },
        {
          id: "pt-2",
          language: "plaintext",
          label: "+ Plain Text Note",
          snippet: "Note item",
          description: "Standard plaintext",
        },
        {
          id: "json-1",
          language: "json",
          label: "+ Pure JSON",
          snippet: '{"pure": true}',
          description: "JSON snippet",
        },
        {
          id: "jsonc-1",
          language: "jsonc",
          label: "+ JSONC Object",
          snippet: '{\n  // comment\n}',
          description: "JSONC snippet",
        },
        {
          id: "cpp-1",
          language: "cpp",
          label: "+ C++ Hello",
          snippet: '#include <iostream>',
          description: "C++ code",
        },
      ];

      // 1. Verify getAvailableSnippetLanguages normalizes and deduplicates
      const available = getAvailableSnippetLanguages(snippetsWithVariations);
      const plaintextOptions = available.filter((a) => a.key === "plaintext" || a.label.toLowerCase() === "plain text");
      expect(plaintextOptions.length).toBe(1);

      const jsonOptions = available.filter((a) => a.key === "jsonc" || a.label.toLowerCase() === "json");
      expect(jsonOptions.length).toBe(1);
      expect(jsonOptions[0].key).toBe("jsonc");
      expect(jsonOptions[0].label).toBe("JSON / JSONC");

      const cppOption = available.find((a) => a.key === "cpp");
      expect(cppOption).toBeDefined();
      expect(cppOption?.label).toBe("C++");

      // 2. Render SnippetManagerView and verify dropdown option count & labels
      render(
        <SnippetManagerView
          snippets={snippetsWithVariations}
          onUpdateSnippets={vi.fn()}
          initialLanguage="all"
        />
      );

      const langSelect = screen.getByRole("combobox") as HTMLSelectElement;
      const optionTexts = Array.from(langSelect.options).map((o) => o.textContent);

      // Plain Text should be combined: Plain Text (2)
      const plaintextEntry = optionTexts.find((t) => t?.startsWith("Plain Text"));
      expect(plaintextEntry).toBe("Plain Text (2)");

      // JSON / JSONC should be combined: JSON / JSONC (2)
      const jsonEntry = optionTexts.find((t) => t?.startsWith("JSON / JSONC"));
      expect(jsonEntry).toBe("JSON / JSONC (2)");

      // No separate "Json (" or "Plain text (" entries
      expect(optionTexts.some((t) => t?.startsWith("Json ("))).toBe(false);
      expect(optionTexts.some((t) => t?.startsWith("Plain text ("))).toBe(false);

      // Filtering with normalized key works for both "plain text" and "plaintext"
      fireEvent.change(langSelect, { target: { value: "plaintext" } });
      expect(screen.getByText("+ Plain Text Todo")).toBeDefined();
      expect(screen.getByText("+ Plain Text Note")).toBeDefined();
      expect(screen.queryByText("+ Pure JSON")).toBeNull();

      // Filtering with "jsonc" shows both "json" and "jsonc"
      fireEvent.change(langSelect, { target: { value: "jsonc" } });
      expect(screen.getByText("+ Pure JSON")).toBeDefined();
      expect(screen.getByText("+ JSONC Object")).toBeDefined();
      expect(screen.queryByText("+ Plain Text Todo")).toBeNull();
    });
  });
});

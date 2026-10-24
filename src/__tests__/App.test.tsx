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
    return Promise.resolve("");
  }),
}));

describe("App component", () => {
  it("renders brand title and responds to dynamic language toggle", async () => {
    await act(async () => {
      await changeLanguage("ja");
    });

    render(<App />);

    // In Japanese
    expect(screen.getByText("FileWorkbench (Cuddle)")).toBeDefined();
    expect(screen.getByText("ワークスペース")).toBeDefined();
    expect(screen.getByTitle("言語切替")).toBeDefined();

    // Toggle to English
    const toggleBtn = screen.getByTitle("言語切替");
    await act(async () => {
      fireEvent.click(toggleBtn);
    });

    // In English
    expect(screen.getByText("FileWorkbench (Cuddle)")).toBeDefined();
    expect(screen.getByText("Workspace")).toBeDefined();
    expect(screen.getByTitle("Language")).toBeDefined();
  });
});

import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom/vitest";
import { render, screen, fireEvent } from "@testing-library/react";
import { ImagePreview } from "../components/ImagePreview";
import type { ImagePreviewDto } from "../types/ImagePreviewDto";

describe("ImagePreview component", () => {
  const sampleRasterPreview: ImagePreviewDto = {
    item_id: "test-img-1",
    raw_base64: "L3BhdGgvdG8vcGhvdG8ucG5n",
    display_name: "photo.png",
    mime_type: "image/png",
    original_format: "PNG",
    data_url: "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==",
    file_size: BigInt(2048),
    is_svg: false,
    svg_content: null,
    is_converted: false,
    exif: {
      make: "Sony",
      model: "ILCE-7M4",
      date_time: "2026-10-25 14:30:00",
      exposure_time: "1/500",
      f_number: "f/2.8",
      iso: "400",
      focal_length: "35mm",
      lens_model: "FE 35mm F1.4 GM",
    },
  };

  const sampleSvgPreview: ImagePreviewDto = {
    item_id: "test-svg-1",
    raw_base64: "L3BhdGgvdG8vaWNvbi5zdmc=",
    display_name: "icon.svg",
    mime_type: "image/svg+xml",
    original_format: "SVG",
    data_url: "data:image/svg+xml;base64,PHN2Zz48Y2lyY2xlIC8+PC9zdmc+",
    file_size: BigInt(512),
    is_svg: true,
    svg_content: `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64"><circle cx="32" cy="32" r="30" fill="teal"/></svg>`,
    is_converted: false,
    exif: null,
  };

  const sampleRawPreview: ImagePreviewDto = {
    item_id: "test-raw-1",
    raw_base64: "L3BhdGgvdG8vY2FtZXJhLmNyMg==",
    display_name: "camera.cr2",
    mime_type: "image/jpeg",
    original_format: "CR2",
    data_url: "data:image/jpeg;base64,/9j/4AAQSkZJRg==",
    file_size: BigInt(25 * 1024 * 1024),
    is_svg: false,
    svg_content: null,
    is_converted: true,
    exif: {
      make: "Canon",
      model: "Canon EOS R5",
      date_time: "2026-10-26 10:15:00",
      exposure_time: "1/1000",
      f_number: "f/1.8",
      iso: "100",
      focal_length: "50mm",
      lens_model: "RF50mm F1.8 STM",
    },
  };

  beforeEach(() => {
    Object.assign(navigator, {
      clipboard: {
        writeText: vi.fn().mockImplementation(() => Promise.resolve()),
      },
    });
  });

  it("renders image preview with metadata badges and image element", () => {
    render(<ImagePreview preview={sampleRasterPreview} />);

    expect(screen.getByText("photo.png")).toBeInTheDocument();
    expect(screen.getByText("PNG")).toBeInTheDocument();
    expect(screen.getByText("2.0 KB")).toBeInTheDocument();

    const img = screen.getByAltText("photo.png");
    expect(img).toBeInTheDocument();
    expect(img).toHaveAttribute("src", sampleRasterPreview.data_url);
  });

  it("renders converted/preview badge for camera RAW preview", () => {
    render(<ImagePreview preview={sampleRawPreview} />);

    expect(screen.getByText("camera.cr2")).toBeInTheDocument();
    expect(screen.getByText("CR2")).toBeInTheDocument();
    expect(screen.getByText("PREVIEW")).toBeInTheDocument();
    expect(screen.getByText("25.00 MB")).toBeInTheDocument();
  });

  it("handles zoom in, zoom out, actual size, and rotation actions", () => {
    render(<ImagePreview preview={sampleRasterPreview} />);

    const zoomInBtn = screen.getByTitle(/Zoom In/i);
    const zoomOutBtn = screen.getByTitle(/Zoom Out/i);
    const resetZoomBtn = screen.getByTitle(/Actual Size/i);
    const rotateBtn = screen.getByTitle(/Rotate/i);

    // Zoom In
    fireEvent.click(zoomInBtn);
    expect(screen.getAllByText(/125%/).length).toBeGreaterThan(0);

    // Rotate
    fireEvent.click(rotateBtn);
    expect(screen.getByText(/90°/)).toBeInTheDocument();

    // Reset Zoom
    fireEvent.click(resetZoomBtn);
    expect(screen.getAllByText(/100%/).length).toBeGreaterThan(0);

    // Zoom Out
    fireEvent.click(zoomOutBtn);
    expect(screen.getAllByText(/80%/).length).toBeGreaterThan(0);
  });

  it("toggles background checkerboard mode", () => {
    render(<ImagePreview preview={sampleRasterPreview} />);
    const viewport = screen.getByTestId("image-preview-viewport");
    expect(viewport).toHaveClass("bg-mode-checker");

    const bgBtn = screen.getByTitle(/Background:/i);
    fireEvent.click(bgBtn);
    expect(viewport).toHaveClass("bg-mode-dark");

    fireEvent.click(bgBtn);
    expect(viewport).toHaveClass("bg-mode-light");

    fireEvent.click(bgBtn);
    expect(viewport).toHaveClass("bg-mode-checker");
  });

  it("toggles EXIF metadata drawer when available", () => {
    render(<ImagePreview preview={sampleRasterPreview} />);

    const exifBtn = screen.getByRole("button", { name: /EXIF/i });
    expect(exifBtn).toBeInTheDocument();

    expect(screen.queryByTestId("image-exif-card")).not.toBeInTheDocument();

    fireEvent.click(exifBtn);
    expect(screen.getByTestId("image-exif-card")).toBeInTheDocument();
    expect(screen.getByText("Sony ILCE-7M4")).toBeInTheDocument();
    expect(screen.getByText("FE 35mm F1.4 GM")).toBeInTheDocument();
    expect(screen.getByText("f/2.8")).toBeInTheDocument();
    expect(screen.getByText("400")).toBeInTheDocument();
  });

  it("supports SVG XML source code toggle", () => {
    render(<ImagePreview preview={sampleSvgPreview} />);

    expect(screen.getByText("icon.svg")).toBeInTheDocument();
    expect(screen.getByText("SVG")).toBeInTheDocument();

    const xmlToggleBtn = screen.getByRole("button", { name: /XML/i });
    expect(xmlToggleBtn).toBeInTheDocument();

    fireEvent.click(xmlToggleBtn);
    const codeView = document.querySelector(".svg-code-view");
    expect(codeView).toBeInTheDocument();

    // Switch back to Rendered
    const imgToggleBtn = screen.getByRole("button", { name: /Image|Rendered/i });
    fireEvent.click(imgToggleBtn);
    expect(screen.getByAltText("icon.svg")).toBeInTheDocument();
  });

  it("copies Data URL to clipboard on copy button click", () => {
    render(<ImagePreview preview={sampleRasterPreview} />);
    const copyBtn = screen.getByTitle(/Copy Data URL/i);
    fireEvent.click(copyBtn);
    expect(navigator.clipboard.writeText).toHaveBeenCalledWith(sampleRasterPreview.data_url);
  });
});

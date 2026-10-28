import React, { useState, useRef, useEffect, useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import {
  ZoomIn,
  ZoomOut,
  RotateCw,
  Maximize2,
  Minimize2,
  RotateCcw,
  Copy,
  Check,
  Image as ImageIcon,
  Grid,
  Sun,
  Moon,
  Code2,
  Eye,
  Camera,
} from "lucide-react";
import type { ImagePreviewDto } from "../types/ImagePreviewDto";
import {
  getImageFormatLabel,
  formatImageFileSize,
  formatAspectRatio,
} from "../utils/imageUtils";
import { highlightCode } from "../utils/syntaxHighlight";

interface ImagePreviewProps {
  preview: ImagePreviewDto;
}

export const ImagePreview: React.FC<ImagePreviewProps> = ({ preview }) => {
  const { t } = useTranslation();

  // Zoom & Pan state
  const [zoom, setZoom] = useState<number>(1.0);
  const [pan, setPan] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [isDragging, setIsDragging] = useState<boolean>(false);
  const [dragStart, setDragStart] = useState<{ x: number; y: number }>({ x: 0, y: 0 });
  const [rotation, setRotation] = useState<number>(0);
  const [fitMode, setFitMode] = useState<boolean>(true);
  const [bgMode, setBgMode] = useState<"checker" | "dark" | "light">("checker");
  const [showExif, setShowExif] = useState<boolean>(false);
  const [svgViewMode, setSvgViewMode] = useState<"rendered" | "code">("rendered");
  const [dimensions, setDimensions] = useState<{ width: number; height: number } | null>(null);
  const [copied, setCopied] = useState<boolean>(false);

  const containerRef = useRef<HTMLDivElement>(null);
  const imgRef = useRef<HTMLImageElement>(null);

  // Reset transforms when preview target changes
  useEffect(() => {
    setZoom(1.0);
    setPan({ x: 0, y: 0 });
    setRotation(0);
    setFitMode(true);
    setShowExif(false);
    setSvgViewMode("rendered");
    setDimensions(null);
  }, [preview.item_id, preview.raw_base64]);

  // Image load handler to get natural resolution
  const handleImageLoad = (e: React.SyntheticEvent<HTMLImageElement>) => {
    const img = e.currentTarget;
    setDimensions({
      width: img.naturalWidth,
      height: img.naturalHeight,
    });
  };

  const handleZoomIn = useCallback(() => {
    setFitMode(false);
    setZoom((prev) => Math.min(prev * 1.25, 20.0));
  }, []);

  const handleZoomOut = useCallback(() => {
    setFitMode(false);
    setZoom((prev) => Math.max(prev / 1.25, 0.05));
  }, []);

  const handleResetZoom = useCallback(() => {
    setFitMode(false);
    setZoom(1.0);
    setPan({ x: 0, y: 0 });
  }, []);

  const handleToggleFit = useCallback(() => {
    setFitMode((prev) => !prev);
    setZoom(1.0);
    setPan({ x: 0, y: 0 });
  }, []);

  const handleRotate = useCallback(() => {
    setRotation((prev) => (prev + 90) % 360);
  }, []);

  const handleToggleBg = useCallback(() => {
    setBgMode((prev) => {
      if (prev === "checker") return "dark";
      if (prev === "dark") return "light";
      return "checker";
    });
  }, []);

  // Mouse wheel zoom
  const handleWheel = (e: React.WheelEvent<HTMLDivElement>) => {
    if (svgViewMode === "code") return;
    e.preventDefault();
    const factor = e.deltaY < 0 ? 1.15 : 1 / 1.15;
    setFitMode(false);
    setZoom((prev) => {
      const next = prev * factor;
      return Math.max(0.05, Math.min(20.0, next));
    });
  };

  // Mouse dragging for pan
  const handleMouseDown = (e: React.MouseEvent<HTMLDivElement>) => {
    if (e.button !== 0 || svgViewMode === "code") return;
    setIsDragging(true);
    setDragStart({ x: e.clientX - pan.x, y: e.clientY - pan.y });
  };

  const handleMouseMove = (e: React.MouseEvent<HTMLDivElement>) => {
    if (!isDragging || svgViewMode === "code") return;
    setPan({
      x: e.clientX - dragStart.x,
      y: e.clientY - dragStart.y,
    });
  };

  const handleMouseUp = () => {
    setIsDragging(false);
  };

  // Double click toggles between fit and 100%
  const handleDoubleClick = () => {
    if (svgViewMode === "code") return;
    handleToggleFit();
  };

  // Copy Data URL or raw SVG text
  const handleCopy = () => {
    if (preview.is_svg && svgViewMode === "code" && preview.svg_content) {
      navigator.clipboard.writeText(preview.svg_content);
    } else {
      navigator.clipboard.writeText(preview.data_url);
    }
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Format label and metadata
  const formatLabel = useMemo(() => {
    if (preview.original_format) {
      return preview.original_format;
    }
    return getImageFormatLabel(preview.mime_type, preview.display_name);
  }, [preview.original_format, preview.mime_type, preview.display_name]);

  const resolutionString = useMemo(() => {
    if (!dimensions) return null;
    const { width, height } = dimensions;
    const mp = ((width * height) / 1_000_000).toFixed(1);
    const mpStr = Number(mp) >= 0.1 ? ` (${mp} MP)` : "";
    return `${width} × ${height}${mpStr}`;
  }, [dimensions]);

  const aspectRatioString = useMemo(() => {
    if (!dimensions) return null;
    return formatAspectRatio(dimensions.width, dimensions.height);
  }, [dimensions]);

  const highlightedSvgHtml = useMemo(() => {
    if (preview.is_svg && preview.svg_content) {
      return highlightCode(preview.svg_content, "html");
    }
    return "";
  }, [preview.is_svg, preview.svg_content]);

  const zoomPercent = Math.round(zoom * 100);

  return (
    <div className="image-preview-container" data-testid="image-preview-container">
      {/* Top Header Bar */}
      <div className="image-preview-bar">
        <div className="image-preview-meta">
          <ImageIcon size={14} className="text-cyan-400" />
          <span className="preview-filename font-mono" title={preview.display_name}>
            {preview.display_name}
          </span>
          <span className="preview-pill img-format-pill">{formatLabel}</span>
          {preview.is_converted && (
            <span
              className="preview-pill img-converted-pill font-mono"
              title={t("image_preview.converted_desc", "埋め込みJPEGまたは外部ツールによる変換プレビューです")}
            >
              PREVIEW
            </span>
          )}
          {resolutionString && (
            <span className="preview-pill img-res-pill" title="Resolution">
              {resolutionString}
            </span>
          )}
          {aspectRatioString && (
            <span className="preview-pill img-ratio-pill" title="Aspect Ratio">
              {aspectRatioString}
            </span>
          )}
          <span className="preview-pill img-size-pill">
            {formatImageFileSize(preview.file_size)}
          </span>
          {!fitMode && (
            <span className="preview-pill img-zoom-pill font-mono">{zoomPercent}%</span>
          )}
        </div>

        <div className="image-preview-actions">
          {/* SVG Rendered / XML Code toggle */}
          {preview.is_svg && preview.svg_content && (
            <div className="preview-mode-toggle">
              <button
                type="button"
                className={`mode-toggle-btn ${svgViewMode === "rendered" ? "active" : ""}`}
                onClick={() => setSvgViewMode("rendered")}
                title={t("image_preview.view_rendered", "Rendered View")}
              >
                <Eye size={12} />
                <span>{t("image_preview.view_rendered", "Image")}</span>
              </button>
              <button
                type="button"
                className={`mode-toggle-btn ${svgViewMode === "code" ? "active" : ""}`}
                onClick={() => setSvgViewMode("code")}
                title={t("image_preview.view_svg_code", "SVG Code")}
              >
                <Code2 size={12} />
                <span>XML</span>
              </button>
            </div>
          )}

          {svgViewMode === "rendered" && (
            <div className="image-control-btn-group">
              <button
                type="button"
                className="panel-header-btn-subtle"
                onClick={handleZoomOut}
                title={t("image_preview.zoom_out", "Zoom Out (-)")}
              >
                <ZoomOut size={13} />
              </button>
              <button
                type="button"
                className={`panel-header-btn-subtle ${!fitMode && zoom === 1.0 ? "active" : ""}`}
                onClick={handleResetZoom}
                title={t("image_preview.zoom_100", "Actual Size (100%)")}
              >
                <RotateCcw size={13} />
              </button>
              <button
                type="button"
                className="panel-header-btn-subtle"
                onClick={handleZoomIn}
                title={t("image_preview.zoom_in", "Zoom In (+)")}
              >
                <ZoomIn size={13} />
              </button>
              <button
                type="button"
                className={`panel-header-btn-subtle ${fitMode ? "active" : ""}`}
                onClick={handleToggleFit}
                title={
                  fitMode
                    ? t("image_preview.fit_window", "Fit to Viewport")
                    : t("image_preview.free_zoom", "Free Zoom")
                }
              >
                {fitMode ? <Minimize2 size={13} /> : <Maximize2 size={13} />}
              </button>
              <button
                type="button"
                className="panel-header-btn-subtle"
                onClick={handleRotate}
                title={t("image_preview.rotate", "Rotate 90° Clockwise")}
              >
                <RotateCw size={13} />
              </button>
              <button
                type="button"
                className="panel-header-btn-subtle"
                onClick={handleToggleBg}
                title={t(
                  "image_preview.toggle_background",
                  `Background: ${bgMode === "checker" ? "Checkerboard" : bgMode === "dark" ? "Dark" : "Light"}`
                )}
              >
                {bgMode === "checker" ? (
                  <Grid size={13} />
                ) : bgMode === "dark" ? (
                  <Moon size={13} />
                ) : (
                  <Sun size={13} />
                )}
              </button>
            </div>
          )}

          {/* EXIF Metadata Button */}
          {preview.exif && (
            <button
              type="button"
              className={`panel-header-btn-subtle ${showExif ? "active" : ""}`}
              onClick={() => setShowExif((prev) => !prev)}
              title={t("image_preview.toggle_exif", "Camera / EXIF Info")}
            >
              <Camera size={13} />
              <span>EXIF</span>
            </button>
          )}

          {/* Copy Button */}
          <button
            type="button"
            className="panel-header-btn-subtle preview-copy-btn"
            onClick={handleCopy}
            title={
              preview.is_svg && svgViewMode === "code"
                ? t("image_preview.copy_svg_code", "Copy SVG XML")
                : t("image_preview.copy_data_url", "Copy Data URL")
            }
          >
            {copied ? <Check size={12} /> : <Copy size={12} />}
            <span>{copied ? t("rich_preview.copied", "Copied") : t("rich_preview.copy", "Copy")}</span>
          </button>
        </div>
      </div>

      {/* Main Preview Content Body */}
      <div className="image-preview-body">
        {preview.is_svg && svgViewMode === "code" ? (
          <div
            className="code-highlighted-view svg-code-view"
            dangerouslySetInnerHTML={{ __html: highlightedSvgHtml }}
          />
        ) : (
          <div
            ref={containerRef}
            className={`image-preview-viewport bg-mode-${bgMode} ${isDragging ? "dragging" : ""}`}
            onWheel={handleWheel}
            onMouseDown={handleMouseDown}
            onMouseMove={handleMouseMove}
            onMouseUp={handleMouseUp}
            onMouseLeave={handleMouseUp}
            onDoubleClick={handleDoubleClick}
            data-testid="image-preview-viewport"
          >
            <div
              className="image-canvas-wrapper"
              style={{
                transform: fitMode
                  ? `rotate(${rotation}deg)`
                  : `translate(${pan.x}px, ${pan.y}px) scale(${zoom}) rotate(${rotation}deg)`,
                cursor: fitMode ? "default" : isDragging ? "grabbing" : "grab",
              }}
            >
              <img
                ref={imgRef}
                src={preview.data_url}
                alt={preview.display_name}
                className={`image-preview-element ${fitMode ? "fit-mode" : "free-mode"}`}
                onLoad={handleImageLoad}
                draggable={false}
              />
            </div>

            {/* Quick floating controls overlay on bottom right */}
            <div className="image-floating-indicator">
              <span>{fitMode ? t("image_preview.fit", "FIT") : `${zoomPercent}%`}</span>
              {rotation !== 0 && <span> • {rotation}°</span>}
            </div>
          </div>
        )}

        {/* EXIF Information Card Overlay / Drawer */}
        {showExif && preview.exif && (
          <div className="image-exif-card" data-testid="image-exif-card">
            <div className="image-exif-header">
              <Camera size={14} className="text-cyan-400" />
              <h5>{t("image_preview.exif_title", "Photo EXIF Information")}</h5>
            </div>
            <div className="image-exif-grid">
              {(preview.exif.make || preview.exif.model) && (
                <div className="exif-item">
                  <span className="exif-label">{t("image_preview.exif_camera", "Camera")}</span>
                  <span className="exif-val">
                    {[preview.exif.make, preview.exif.model].filter(Boolean).join(" ")}
                  </span>
                </div>
              )}
              {preview.exif.lens_model && (
                <div className="exif-item">
                  <span className="exif-label">{t("image_preview.exif_lens", "Lens")}</span>
                  <span className="exif-val">{preview.exif.lens_model}</span>
                </div>
              )}
              {preview.exif.date_time && (
                <div className="exif-item">
                  <span className="exif-label">{t("image_preview.exif_date", "Date / Time")}</span>
                  <span className="exif-val font-mono">{preview.exif.date_time}</span>
                </div>
              )}
              {preview.exif.f_number && (
                <div className="exif-item">
                  <span className="exif-label">{t("image_preview.exif_aperture", "Aperture")}</span>
                  <span className="exif-val font-mono">{preview.exif.f_number}</span>
                </div>
              )}
              {preview.exif.exposure_time && (
                <div className="exif-item">
                  <span className="exif-label">{t("image_preview.exif_shutter", "Shutter")}</span>
                  <span className="exif-val font-mono">{preview.exif.exposure_time}s</span>
                </div>
              )}
              {preview.exif.iso && (
                <div className="exif-item">
                  <span className="exif-label">ISO</span>
                  <span className="exif-val font-mono">{preview.exif.iso}</span>
                </div>
              )}
              {preview.exif.focal_length && (
                <div className="exif-item">
                  <span className="exif-label">{t("image_preview.exif_focal", "Focal Length")}</span>
                  <span className="exif-val font-mono">{preview.exif.focal_length}</span>
                </div>
              )}
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

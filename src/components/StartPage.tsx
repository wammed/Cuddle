// src/components/StartPage.tsx
// Cuddle - Start Dashboard & Quick Workflow Launchpad

import React from "react";
import { useTranslation } from "react-i18next";
import {
  FolderOpen,
  Workflow,
  Sparkles,
  Settings,
  ShieldCheck,
  HardDrive,
  Home,
  ArrowRight,
  Database,
  Lock,
} from "lucide-react";
import type { SystemPathsDto } from "../types/SystemPathsDto";

interface StartPageProps {
  onOpenFolder: () => void;
  onNavigatePath: (path: string) => void;
  onOpenPipeline: () => void;
  onOpenBulkRename: () => void;
  onOpenSettings: () => void;
  onSwitchToWorkbench: () => void;
  systemPaths: SystemPathsDto | null;
  landlockSupported: boolean;
  landlockAbi: string | null;
  stagedCount: number;
}

export const StartPage: React.FC<StartPageProps> = ({
  onOpenFolder,
  onNavigatePath,
  onOpenPipeline,
  onOpenBulkRename,
  onOpenSettings,
  onSwitchToWorkbench,
  systemPaths,
  landlockSupported,
  landlockAbi,
  stagedCount,
}) => {
  const { t } = useTranslation();

  return (
    <div className="start-page-container">
      <div className="start-page-inner">
        {/* Hero Section */}
        <div className="start-hero-card">
          <div className="start-hero-top">
            <img src="/Cuddle-Icon.svg" alt="Cuddle Logo" className="start-hero-logo" />
            <div className="start-hero-headings">
              <div className="start-hero-title-row">
                <h2>{t("app.title")}</h2>
                <span className="start-version-pill">v0.1.0</span>
                <span className="start-secure-pill">
                  <ShieldCheck size={13} />
                  <span>{t("workspace.original_protected")}</span>
                </span>
              </div>
              <h3 className="start-hero-subtitle">{t("start.hero_title")}</h3>
              <p className="start-hero-desc">{t("start.hero_subtitle")}</p>
            </div>
          </div>

          <div className="start-hero-actions">
            <button
              type="button"
              className="start-primary-btn"
              onClick={onSwitchToWorkbench}
            >
              <span>{t("start.switch_workbench")}</span>
              <ArrowRight size={15} />
            </button>
            <button
              type="button"
              className="start-secondary-btn"
              onClick={onOpenSettings}
            >
              <Settings size={15} />
              <span>{t("start.btn_settings")}</span>
            </button>
          </div>
        </div>

        {/* 3 Main Action Cards */}
        <div className="start-section-heading">
          <h4>{t("start.features_title")}</h4>
        </div>

        <div className="start-action-grid">
          {/* Card 1: Open Folder & Browse */}
          <div className="start-feature-card" onClick={onOpenFolder}>
            <div className="feature-card-icon-wrap folder">
              <FolderOpen size={24} />
            </div>
            <div className="feature-card-body">
              <h5>{t("start.btn_open_folder")}</h5>
              <p>高速ファイル走査・ツリー探索・非UTF-8安全表示</p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">ブラウザーを開く</span>
              <ArrowRight size={13} />
            </div>
          </div>

          {/* Card 2: 30-Pipeline Runner */}
          <div className="start-feature-card" onClick={onOpenPipeline}>
            <div className="feature-card-icon-wrap pipeline">
              <Workflow size={24} />
            </div>
            <div className="feature-card-body">
              <h5>{t("start.btn_pipeline")}</h5>
              <p>WebP変換、動画GIF、PDF結合、秘密情報マスキング</p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">プリセット一覧</span>
              <ArrowRight size={13} />
            </div>
          </div>

          {/* Card 3: Bulk Rename & Normalization */}
          <div className="start-feature-card" onClick={onOpenBulkRename}>
            <div className="feature-card-icon-wrap rename">
              <Sparkles size={24} />
            </div>
            <div className="feature-card-body">
              <h5>{t("start.btn_bulk_rename")}</h5>
              <p>正規表現置換、連番ゼロ埋め、Unicode/全半角正規化</p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">リネーム起動</span>
              <ArrowRight size={13} />
            </div>
          </div>
        </div>

        {/* Quick Access & System Status Row */}
        <div className="start-split-row">
          {/* Quick Access Locations */}
          <div className="start-card-panel">
            <div className="card-panel-header">
              <Home size={15} className="text-teal-400" />
              <h5>{t("start.recent_locations")}</h5>
            </div>
            <div className="quick-locations-list">
              {systemPaths?.home_dir && (
                <button
                  type="button"
                  className="quick-loc-item"
                  onClick={() => {
                    onNavigatePath(systemPaths.home_dir);
                    onSwitchToWorkbench();
                  }}
                >
                  <Home size={14} className="text-teal-400" />
                  <span className="font-mono">{systemPaths.home_dir}</span>
                  <span className="loc-tag">Home</span>
                </button>
              )}
              {systemPaths?.current_dir && (
                <button
                  type="button"
                  className="quick-loc-item"
                  onClick={() => {
                    onNavigatePath(systemPaths.current_dir);
                    onSwitchToWorkbench();
                  }}
                >
                  <FolderOpen size={14} className="text-teal-400" />
                  <span className="font-mono">{systemPaths.current_dir}</span>
                  <span className="loc-tag">Project</span>
                </button>
              )}
            </div>
          </div>

          {/* System Security & Health Monitor */}
          <div className="start-card-panel">
            <div className="card-panel-header">
              <ShieldCheck size={15} className="text-emerald-400" />
              <h5>{t("start.status_title")}</h5>
            </div>
            <div className="status-indicators-grid">
              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <ShieldCheck size={13} className="text-teal-400" />
                  <span>Landlock LSM</span>
                </div>
                <span className="indicator-val-pill active">
                  {landlockSupported ? `ABI ${landlockAbi || "V5"} 稼働中` : "未検出"}
                </span>
              </div>

              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <HardDrive size={13} className="text-emerald-400" />
                  <span>空き容量保護</span>
                </div>
                <span className="indicator-val-pill active">1.5x ガードレール</span>
              </div>

              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <Database size={13} className="text-teal-400" />
                  <span>作業コピー</span>
                </div>
                <span className="indicator-val-pill">
                  {stagedCount} 件ステージング中
                </span>
              </div>

              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <Lock size={13} className="text-teal-400" />
                  <span>原本保護方針</span>
                </div>
                <span className="indicator-val-pill active">Read-Only 原本</span>
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
};

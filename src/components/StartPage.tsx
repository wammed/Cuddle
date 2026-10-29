// src/components/StartPage.tsx
// Cuddle - Start Dashboard & Quick Workflow Launchpad

import React, { useState, useEffect } from "react";
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
  Archive,
  RotateCcw,
  X,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import type { SystemPathsDto } from "../types/SystemPathsDto";
import type { BackupArchiveInfoDto } from "../types/BackupArchiveInfoDto";
import { formatBytes, formatTimestamp } from "../utils/format";

interface StartPageProps {
  onOpenFolder: () => void;
  onNavigatePath: (path: string) => void;
  onOpenPipeline: () => void;
  onOpenBulkRename: () => void;
  onOpenSettings: () => void;
  onSwitchToWorkbench: () => void;
  onResumeBackup?: (archivePath?: string) => Promise<void>;
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
  onResumeBackup,
  systemPaths,
  landlockSupported,
  landlockAbi,
  stagedCount,
}) => {
  const { t } = useTranslation();
  const [backups, setBackups] = useState<BackupArchiveInfoDto[]>([]);
  const [isRestoring, setIsRestoring] = useState(false);
  const [showSelectModal, setShowSelectModal] = useState(false);

  const loadBackups = async () => {
    try {
      const list = await invoke<BackupArchiveInfoDto[]>("list_workspace_backups");
      setBackups(Array.isArray(list) ? list : []);
    } catch (err) {
      console.warn("Failed to list backups on start page:", err);
      setBackups([]);
    }
  };

  useEffect(() => {
    loadBackups();
  }, []);

  const handleDirectResume = async (archivePath?: string) => {
    if (!onResumeBackup) return;
    const target = archivePath ? backups.find((b) => b.full_path === archivePath) : backups[0] || null;
    if (!target && !archivePath) {
      alert(t("start.no_backups_found"));
      return;
    }
    const filename = target ? target.filename : "backup";
    const count = target?.file_count ?? 1;
    if (window.confirm(t("start.resume_backup_confirm", { filename, count }))) {
      try {
        setIsRestoring(true);
        await onResumeBackup(archivePath);
        setShowSelectModal(false);
      } finally {
        setIsRestoring(false);
      }
    }
  };

  // Mouse tracking spotlight feedback (Phase 10 / Task 2-7)
  const handleCardMouseMove = (e: React.MouseEvent<HTMLElement>) => {
    const rect = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    e.currentTarget.style.setProperty("--mouse-x", `${x}px`);
    e.currentTarget.style.setProperty("--mouse-y", `${y}px`);
  };

  return (
    <div className="start-page-container">
      <div className="start-page-inner">
        {/* Hero Section */}
        <div className="start-hero-card spotlight-card" onMouseMove={handleCardMouseMove}>
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
            {onResumeBackup && (
              <button
                type="button"
                className={`start-resume-btn ${backups.length > 0 ? "active" : "disabled"}`}
                onClick={() => {
                  if (backups.length === 0) {
                    alert(t("start.no_backups_found"));
                  } else if (backups.length === 1) {
                    handleDirectResume(backups[0].full_path);
                  } else {
                    setShowSelectModal(true);
                  }
                }}
                disabled={isRestoring}
                title={
                  backups.length > 0
                    ? `${t("start.btn_resume_backup")} (${backups[0].filename})`
                    : t("start.no_backups_found")
                }
              >
                <Archive size={15} className="text-teal-400" />
                <span>{t("start.btn_resume_backup")}</span>
                {backups.length > 0 && (
                  <span className="resume-badge font-mono">
                    {backups[0].file_count != null ? `${backups[0].file_count}` : "1"}
                  </span>
                )}
              </button>
            )}
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
          <div className="start-feature-card spotlight-card" onMouseMove={handleCardMouseMove} onClick={onOpenFolder}>
            <div className="feature-card-icon-wrap folder">
              <FolderOpen size={24} />
            </div>
            <div className="feature-card-body">
              <h5>{t("start.btn_open_folder")}</h5>
              <p>{t("start.feature_browser_desc")}</p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">{t("start.feature_browser_link")}</span>
              <ArrowRight size={13} />
            </div>
          </div>

          {/* Card 2: 30-Pipeline Runner */}
          <div className="start-feature-card spotlight-card" onMouseMove={handleCardMouseMove} onClick={onOpenPipeline}>
            <div className="feature-card-icon-wrap pipeline">
              <Workflow size={24} />
            </div>
            <div className="feature-card-body">
              <h5>{t("start.btn_pipeline")}</h5>
              <p>{t("start.feature_pipeline_desc")}</p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">{t("start.feature_pipeline_link")}</span>
              <ArrowRight size={13} />
            </div>
          </div>

          {/* Card 3: Bulk Rename & Normalization */}
          <div className="start-feature-card spotlight-card" onMouseMove={handleCardMouseMove} onClick={onOpenBulkRename}>
            <div className="feature-card-icon-wrap rename">
              <Sparkles size={24} />
            </div>
            <div className="feature-card-body">
              <h5>{t("start.btn_bulk_rename")}</h5>
              <p>{t("start.feature_rename_desc")}</p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">{t("start.feature_rename_link")}</span>
              <ArrowRight size={13} />
            </div>
          </div>

          {/* Card 4: Resume from Backup */}
          <div
            className={`start-feature-card spotlight-card ${backups.length === 0 ? "opacity-75" : ""}`}
            onMouseMove={handleCardMouseMove}
            onClick={() => {
              if (backups.length === 0) {
                alert(t("start.no_backups_found"));
              } else if (backups.length === 1) {
                handleDirectResume(backups[0].full_path);
              } else {
                setShowSelectModal(true);
              }
            }}
          >
            <div className="feature-card-icon-wrap backup">
              <Archive size={24} />
            </div>
            <div className="feature-card-body">
              <div className="flex items-center justify-between">
                <h5>{t("start.btn_resume_backup")}</h5>
                {backups.length > 0 && (
                  <span className="text-[10px] font-mono px-1.5 py-0.5 rounded bg-teal-500/20 text-teal-300 border border-teal-500/30">
                    {backups.length}
                  </span>
                )}
              </div>
              <p>
                {backups.length > 0
                  ? `${backups[0].filename} (${formatBytes(Number(backups[0].size_bytes))})`
                  : t("start.resume_backup_desc")}
              </p>
            </div>
            <div className="feature-card-footer">
              <span className="feature-link-text">
                {backups.length > 0 ? t("start.btn_resume_backup") : t("start.no_backups_found")}
              </span>
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
                  <span className="loc-tag">{t("start.loc_home")}</span>
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
                  <span className="loc-tag">{t("start.loc_project")}</span>
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
                  <span>{t("start.status_landlock")}</span>
                </div>
                <span className="indicator-val-pill active">
                  {landlockSupported
                    ? t("start.status_landlock_active", { abi: landlockAbi || "V5" })
                    : t("start.status_landlock_inactive")}
                </span>
              </div>

              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <HardDrive size={13} className="text-emerald-400" />
                  <span>{t("start.status_disk_guard")}</span>
                </div>
                <span className="indicator-val-pill active">{t("start.status_disk_guard_val")}</span>
              </div>

              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <Database size={13} className="text-teal-400" />
                  <span>{t("start.status_staging")}</span>
                </div>
                <span className="indicator-val-pill">
                  {t("start.status_staging_count", { count: stagedCount })}
                </span>
              </div>

              <div className="status-indicator-box">
                <div className="indicator-title-row">
                  <Lock size={13} className="text-teal-400" />
                  <span>{t("start.status_original_policy")}</span>
                </div>
                <span className="indicator-val-pill active">{t("start.status_original_policy_val")}</span>
              </div>
            </div>
          </div>
        </div>
      </div>

      {/* Select Backup Archive Modal */}
      {showSelectModal && (
        <div className="modal-backdrop" onClick={() => setShowSelectModal(false)}>
          <div
            className="exit-backup-dialog"
            onClick={(e) => e.stopPropagation()}
            style={{ maxWidth: "560px" }}
          >
            <div className="dialog-header">
              <div className="dialog-title-group">
                <Archive size={18} className="text-teal-400" />
                <h4>{t("start.select_backup_to_restore")}</h4>
              </div>
              <button
                type="button"
                className="panel-header-btn"
                onClick={() => setShowSelectModal(false)}
                title={t("common.close")}
              >
                <X size={14} />
              </button>
            </div>

            <div className="dialog-body">
              <p className="text-xs text-slate-300 mb-3">
                {t("start.resume_backup_desc")}
              </p>

              <div className="existing-backups-list max-h-64 overflow-y-auto space-y-2">
                {backups.map((bkp) => (
                  <div
                    key={bkp.filename}
                    className="flex items-center justify-between p-2.5 rounded bg-black/40 border border-white/10 hover:border-teal-500/50 transition-colors"
                  >
                    <div className="flex flex-col min-w-0 mr-3">
                      <span className="font-mono text-xs font-semibold text-slate-100 truncate">
                        {bkp.filename}
                      </span>
                      <span className="text-[11px] text-slate-400 mt-0.5">
                        {formatTimestamp(Number(bkp.created_at_ms))} • {formatBytes(Number(bkp.size_bytes))}
                        {bkp.file_count != null ? ` • ${t("backup.files_count", { count: bkp.file_count })}` : ""}
                      </span>
                    </div>

                    <button
                      type="button"
                      className="start-primary-btn !py-1 !px-3 !text-xs shrink-0"
                      onClick={() => handleDirectResume(bkp.full_path)}
                      disabled={isRestoring}
                    >
                      <RotateCcw size={12} />
                      <span>{t("start.btn_resume_backup")}</span>
                    </button>
                  </div>
                ))}
              </div>
            </div>

            <div className="dialog-footer">
              <button
                type="button"
                className="dialog-btn secondary"
                onClick={() => setShowSelectModal(false)}
              >
                {t("common.close")}
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

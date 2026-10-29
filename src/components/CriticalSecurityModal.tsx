import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { ShieldAlert, AlertTriangle, ShieldCheck, LogOut } from "lucide-react";

interface CriticalSecurityModalProps {
  isOpen: boolean;
  onConsent: (remember: boolean) => void;
  onExit: () => void;
}

export const CriticalSecurityModal: React.FC<CriticalSecurityModalProps> = ({
  isOpen,
  onConsent,
  onExit,
}) => {
  const { t } = useTranslation();
  const [rememberChoice, setRememberChoice] = useState(false);

  if (!isOpen) return null;

  return (
    <div className="modal-backdrop critical-security-backdrop" style={{ zIndex: 99999 }}>
      <div className="critical-security-dialog max-w-xl w-full mx-4 rounded-xl p-6 bg-[#0a0f16] border border-rose-500/40 shadow-2xl backdrop-blur-2xl">
        {/* Header */}
        <div className="flex items-center gap-3.5 mb-4 pb-3 border-b border-rose-500/20">
          <div className="p-2.5 rounded-lg bg-rose-500/10 border border-rose-500/30 text-rose-400">
            <ShieldAlert size={26} />
          </div>
          <div>
            <h3 className="text-base font-bold text-rose-300 tracking-wide">
              {t("security_modal.title")}
            </h3>
            <span className="text-[11px] font-mono text-rose-400/80">
              Linux Landlock LSM: Not Enforced
            </span>
          </div>
        </div>

        {/* Content Body */}
        <div className="space-y-3.5 text-xs text-slate-300 leading-relaxed">
          <p>{t("security_modal.desc_p1")}</p>
          <p>{t("security_modal.desc_p2")}</p>

          <div className="p-3.5 rounded-lg bg-amber-950/20 border border-amber-500/20 space-y-2">
            <div className="flex items-center gap-2 text-amber-400 font-semibold text-xs">
              <AlertTriangle size={15} />
              <span>{t("security_modal.risks_title")}</span>
            </div>
            <ul className="list-disc list-inside space-y-1 text-slate-300 text-[11px]">
              <li>{t("security_modal.risk_1")}</li>
              <li className="text-emerald-400/90">{t("security_modal.risk_2")}</li>
            </ul>
          </div>

          <label className="flex items-center gap-2.5 pt-1 text-xs text-slate-400 cursor-pointer select-none">
            <input
              type="checkbox"
              checked={rememberChoice}
              onChange={(e) => setRememberChoice(e.target.checked)}
              className="rounded accent-teal-500 cursor-pointer"
            />
            <span>{t("security_modal.remember_choice")}</span>
          </label>
        </div>

        {/* Actions Footer */}
        <div className="flex items-center justify-end gap-3 mt-6 pt-4 border-t border-white/10">
          <button
            type="button"
            className="px-4 py-2 rounded-lg text-xs font-semibold text-rose-300 hover:text-rose-200 bg-rose-950/40 hover:bg-rose-900/60 border border-rose-500/30 transition-all flex items-center gap-1.5"
            onClick={onExit}
          >
            <LogOut size={14} />
            <span>{t("security_modal.btn_exit")}</span>
          </button>

          <button
            type="button"
            className="px-4 py-2 rounded-lg text-xs font-semibold text-teal-200 hover:text-white bg-teal-600/80 hover:bg-teal-500 border border-teal-400/50 shadow-lg shadow-teal-950/50 transition-all flex items-center gap-1.5"
            onClick={() => onConsent(rememberChoice)}
          >
            <ShieldCheck size={14} />
            <span>{t("security_modal.btn_proceed")}</span>
          </button>
        </div>
      </div>
    </div>
  );
};

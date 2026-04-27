import { memo } from "react";
import { Download, LoaderCircle } from "lucide-react";
import { formatBytes } from "../lib/helpers";

export type UpdatePhase = "checking" | "downloading" | "installing" | "relaunching";

interface UpdateProgressModalProps {
  phase: UpdatePhase;
  downloadedBytes: number;
  totalBytes: number | null;
  onCancel: () => void;
}

const PHASE_LABELS: Record<UpdatePhase, string> = {
  checking: "正在检查更新…",
  downloading: "正在下载更新",
  installing: "正在安装更新…",
  relaunching: "正在重启应用…"
};

export const UpdateProgressModal = memo(function UpdateProgressModal({
  phase,
  downloadedBytes,
  totalBytes,
  onCancel
}: UpdateProgressModalProps) {
  const hasTotal = totalBytes != null && totalBytes > 0;
  const percent = hasTotal
    ? Math.max(0, Math.min(100, Math.floor((downloadedBytes / totalBytes) * 100)))
    : null;

  const progressText = hasTotal
    ? `${formatBytes(downloadedBytes)} / ${formatBytes(totalBytes)}`
    : `已下载 ${formatBytes(downloadedBytes)}`;

  return (
    <div className="modal-backdrop" onClick={onCancel}>
      <div className="modal-card update-progress-card" onClick={(e) => e.stopPropagation()}>
        <div className="update-progress-header">
          <h3>版本更新</h3>
        </div>

        <div className="update-progress-body">
          {phase === "checking" || phase === "relaunching" || phase === "installing" ? (
            <div className="update-progress-spinner">
              <LoaderCircle className="spin" />
              <span>{PHASE_LABELS[phase]}</span>
            </div>
          ) : (
            <div className="update-progress-bars">
              <div className="update-progress-info">
                <Download />
                <span>{PHASE_LABELS[phase]}</span>
              </div>
              <div className="update-progress-track">
                <div
                  className="update-progress-fill"
                  style={{ width: `${percent ?? 0}%` }}
                />
              </div>
              <div className="update-progress-data">
                <span>{percent != null ? `${percent}%` : "—"}</span>
                <span>{progressText}</span>
              </div>
            </div>
          )}
        </div>

        <div className="update-progress-footer">
          <button
            type="button"
            className="button button-secondary"
            onClick={onCancel}
          >
            取消
          </button>
        </div>
      </div>
    </div>
  );
});

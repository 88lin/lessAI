import { memo, useEffect, useMemo, useState } from "react";
import { Check, Orbit, X } from "lucide-react";
import type { AppSettings, ProviderCheckResult } from "../lib/types";
import type { NoticeTone } from "../lib/constants";
import { MODE_OPTIONS, PRESET_OPTIONS } from "../lib/constants";
import { PROMPT_PRESETS, getPromptPresetDefinition } from "../lib/promptPresets";
import { isSettingsReady } from "../lib/helpers";
import { ActionButton } from "./ActionButton";
import { StatusBadge } from "./StatusBadge";

type SettingsPage = "provider" | "strategy" | "prompt";

interface SettingsModalProps {
  open: boolean;
  settings: AppSettings;
  providerStatus: ProviderCheckResult | null;
  busyAction: string | null;
  onClose: () => void;
  onUpdateStringSetting: <K extends "baseUrl" | "apiKey" | "model">(
    key: K,
    value: string
  ) => void;
  onUpdateNumberSetting: (key: "timeoutMs" | "temperature", value: string) => void;
  onUpdateChunkPreset: (value: AppSettings["chunkPreset"]) => void;
  onUpdateRewriteMode: (value: AppSettings["rewriteMode"]) => void;
  onUpdatePromptPresetId: (value: AppSettings["promptPresetId"]) => void;
  onTestProvider: () => void;
  onSaveSettings: () => void;
}

export const SettingsModal = memo(function SettingsModal({
  open,
  settings,
  providerStatus,
  busyAction,
  onClose,
  onUpdateStringSetting,
  onUpdateNumberSetting,
  onUpdateChunkPreset,
  onUpdateRewriteMode,
  onUpdatePromptPresetId,
  onTestProvider,
  onSaveSettings
}: SettingsModalProps) {
  const [page, setPage] = useState<SettingsPage>("provider");
  const [showPromptPreview, setShowPromptPreview] = useState(false);

  useEffect(() => {
    if (!open) return;
    function handleKeyDown(event: KeyboardEvent) {
      if (event.key === "Escape") {
        onClose();
      }
    }
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [open, onClose]);

  useEffect(() => {
    if (!open) return;
    // 每次打开设置，默认落在连接配置页，并收起提示词预览，减少干扰。
    setPage("provider");
    setShowPromptPreview(false);
  }, [open]);

  const providerTone: NoticeTone =
    providerStatus == null ? "info" : providerStatus.ok ? "success" : "warning";

  const settingsReady = useMemo(() => isSettingsReady(settings), [settings]);

  const selectedPrompt = useMemo(
    () => getPromptPresetDefinition(settings.promptPresetId),
    [settings.promptPresetId]
  );

  if (!open) return null;

  return (
    <div
      className="modal-overlay"
      role="dialog"
      aria-modal="true"
      aria-label="设置"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <div className="modal-card">
        <header className="modal-header">
          <div className="modal-header-title">
            <h2>设置</h2>
            <p className="modal-subtitle">
              连接、改写策略、提示词都在这里统一管理
            </p>
          </div>
          <button
            type="button"
            className="icon-button"
            onClick={onClose}
            aria-label="关闭设置"
            title="关闭"
          >
            <X />
          </button>
        </header>

        <div className="modal-body">
          <nav className="settings-nav" aria-label="设置分类">
            <button
              type="button"
              className={`settings-nav-item ${page === "provider" ? "is-active" : ""}`}
              onClick={() => setPage("provider")}
            >
              <strong>模型与接口</strong>
              <span>Base URL / Key / Model</span>
            </button>
            <button
              type="button"
              className={`settings-nav-item ${page === "strategy" ? "is-active" : ""}`}
              onClick={() => setPage("strategy")}
            >
              <strong>改写策略</strong>
              <span>切段 / 默认执行模式</span>
            </button>
            <button
              type="button"
              className={`settings-nav-item ${page === "prompt" ? "is-active" : ""}`}
              onClick={() => setPage("prompt")}
            >
              <strong>提示词</strong>
              <span>从 prompt/ 加载</span>
            </button>
          </nav>

          <section className="settings-content" aria-label="设置内容">
            {page === "provider" ? (
              <div className="settings-page">
                <div className="settings-page-head">
                  <h3>模型与接口</h3>
                  <StatusBadge tone={providerTone}>
                    {providerStatus
                      ? providerStatus.ok
                        ? "连接正常"
                        : "待修正"
                      : "未测试"}
                  </StatusBadge>
                </div>

                <div className="field-grid">
                  <label className="field">
                    <span>Base URL</span>
                    <input
                      value={settings.baseUrl}
                      onChange={(event) =>
                        onUpdateStringSetting("baseUrl", event.target.value)
                      }
                      placeholder="https://api.openai.com/v1"
                    />
                  </label>
                  <label className="field">
                    <span>API Key</span>
                    <input
                      type="password"
                      value={settings.apiKey}
                      onChange={(event) =>
                        onUpdateStringSetting("apiKey", event.target.value)
                      }
                      placeholder="sk-..."
                    />
                  </label>
                  <label className="field">
                    <span>Model</span>
                    <input
                      value={settings.model}
                      onChange={(event) =>
                        onUpdateStringSetting("model", event.target.value)
                      }
                      placeholder="gpt-4.1-mini"
                    />
                  </label>
                  <label className="field field-inline">
                    <span>超时（毫秒）</span>
                    <input
                      type="number"
                      min={1000}
                      step={1000}
                      value={settings.timeoutMs}
                      onChange={(event) =>
                        onUpdateNumberSetting("timeoutMs", event.target.value)
                      }
                    />
                  </label>
                </div>

                <div className="field-block">
                  <div className="field-line">
                    <span>Temperature</span>
                    <strong>{settings.temperature.toFixed(1)}</strong>
                  </div>
                  <input
                    type="range"
                    min={0}
                    max={2}
                    step={0.1}
                    value={settings.temperature}
                    onChange={(event) =>
                      onUpdateNumberSetting("temperature", event.target.value)
                    }
                  />
                </div>

                {providerStatus ? (
                  <div className="empty-inline">
                    <span>{providerStatus.message}</span>
                  </div>
                ) : null}
              </div>
            ) : null}

            {page === "strategy" ? (
              <div className="settings-page">
                <div className="settings-page-head">
                  <h3>改写策略</h3>
                  <StatusBadge tone={settingsReady ? "success" : "warning"}>
                    {settingsReady ? "可执行" : "未配置"}
                  </StatusBadge>
                </div>

                <div className="field-block">
                  <div className="field-line">
                    <span>默认切段策略</span>
                    <strong>
                      {PRESET_OPTIONS.find((item) => item.value === settings.chunkPreset)
                        ?.label}
                    </strong>
                  </div>
                  <div className="segmented-grid">
                    {PRESET_OPTIONS.map((option) => (
                      <button
                        key={option.value}
                        type="button"
                        className={`segment-card ${
                          settings.chunkPreset === option.value ? "is-active" : ""
                        }`}
                        onClick={() => onUpdateChunkPreset(option.value)}
                      >
                        <strong>{option.label}</strong>
                        <span>{option.hint}</span>
                      </button>
                    ))}
                  </div>
                </div>

                <div className="field-block">
                  <div className="field-line">
                    <span>默认执行模式</span>
                    <strong>
                      {MODE_OPTIONS.find((item) => item.value === settings.rewriteMode)
                        ?.label}
                    </strong>
                  </div>
                  <div className="segmented-grid">
                    {MODE_OPTIONS.map((option) => (
                      <button
                        key={option.value}
                        type="button"
                        className={`segment-card ${
                          settings.rewriteMode === option.value ? "is-active" : ""
                        }`}
                        onClick={() => onUpdateRewriteMode(option.value)}
                      >
                        <strong>{option.label}</strong>
                        <span>{option.hint}</span>
                      </button>
                    ))}
                  </div>
                </div>
              </div>
            ) : null}

            {page === "prompt" ? (
              <div className="settings-page">
                <div className="settings-page-head">
                  <h3>提示词（降低 AIGC 痕迹）</h3>
                  <StatusBadge tone="info">{PROMPT_PRESETS.length} 个预设</StatusBadge>
                </div>

                <div className="prompt-preset-grid">
                  {PROMPT_PRESETS.map((preset) => (
                    <button
                      key={preset.id}
                      type="button"
                      className={`segment-card prompt-preset-card ${
                        settings.promptPresetId === preset.id ? "is-active" : ""
                      }`}
                      onClick={() => onUpdatePromptPresetId(preset.id)}
                    >
                      <strong>{preset.label}</strong>
                      <span>{preset.hint}</span>
                    </button>
                  ))}
                </div>

                <div className="assistant-inline-actions">
                  <button
                    type="button"
                    className={`switch-chip ${showPromptPreview ? "is-active" : ""}`}
                    onClick={() => setShowPromptPreview((current) => !current)}
                  >
                    {showPromptPreview ? "收起预览" : "预览提示词"}
                  </button>
                </div>

                {showPromptPreview ? (
                  <label className="field">
                    <span>当前选择：{selectedPrompt.label}</span>
                    <textarea
                      className="prompt-preview"
                      value={selectedPrompt.content.trim()}
                      readOnly
                    />
                  </label>
                ) : (
                  <div className="empty-inline">
                    <span>
                      当前选择：<strong>{selectedPrompt.label}</strong>
                    </span>
                  </div>
                )}
              </div>
            ) : null}
          </section>
        </div>

        <footer className="modal-footer">
          <div className="modal-footer-left">
            <StatusBadge tone={settingsReady ? "success" : "warning"}>
              {settingsReady ? "设置已就绪" : "需要配置 Base URL / Key / Model"}
            </StatusBadge>
          </div>

          <div className="modal-footer-actions">
            {page === "provider" ? (
              <ActionButton
                icon={Orbit}
                label="测试连接"
                busy={busyAction === "test-provider"}
                disabled={Boolean(busyAction) && busyAction !== "test-provider"}
                onClick={onTestProvider}
                variant="secondary"
              />
            ) : null}
            <ActionButton
              icon={Check}
              label="保存配置"
              busy={busyAction === "save-settings"}
              disabled={Boolean(busyAction) && busyAction !== "save-settings"}
              onClick={onSaveSettings}
              variant="primary"
            />
          </div>
        </footer>
      </div>
    </div>
  );
});

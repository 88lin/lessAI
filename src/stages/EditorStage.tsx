import { memo, useEffect, useMemo } from "react";
import {
  ArrowLeft,
  Save,
  Undo2
} from "lucide-react";
import type { DocumentSession } from "../lib/types";
import { ActionButton } from "../components/ActionButton";
import { countCharacters } from "../lib/helpers";

interface EditorStageProps {
  session: DocumentSession;
  text: string;
  dirty: boolean;
  busyAction: string | null;
  onChangeText: (value: string) => void;
  onSave: () => void;
  onSaveAndBack: () => void;
  onDiscard: () => void;
  onBack: () => void;
}

export const EditorStage = memo(function EditorStage({
  session,
  text,
  dirty,
  busyAction,
  onChangeText,
  onSave,
  onSaveAndBack,
  onDiscard,
  onBack
}: EditorStageProps) {
  const anyBusy = Boolean(busyAction);
  const saveBusy = busyAction === "save-edits";
  const saveAndBackBusy = busyAction === "save-edits-and-back";

  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      const saveCombo = (event.ctrlKey || event.metaKey) && key === "s";
      if (!saveCombo) return;

      event.preventDefault();
      if (!dirty || anyBusy) return;
      onSave();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [anyBusy, dirty, onSave]);

  const currentCount = useMemo(() => countCharacters(text), [text]);

  return (
    <div className="editor-stage">
      <div className="editor-toolbar">
        <div className="editor-toolbar-main">
          <p>EDITOR</p>
          <h3>{session.title}</h3>
        </div>
        <div className="editor-toolbar-meta" aria-label="编辑器信息与操作">
          <span className="editor-chip">{dirty ? "未保存" : "已保存"}</span>
          <span className="editor-chip" title="字符数（不含空白）">
            字符：{currentCount}
          </span>

          <ActionButton
            icon={Save}
            label="保存并返回工作台"
            busy={saveAndBackBusy}
            disabled={!dirty || (anyBusy && !saveAndBackBusy)}
            onClick={onSaveAndBack}
            variant="primary"
            className="editor-toolbar-action"
          />

          <button
            type="button"
            className="icon-button"
            onClick={onSave}
            aria-label="保存"
            title={dirty ? "保存（Ctrl/Cmd+S）" : "没有修改，无需保存"}
            disabled={!dirty || anyBusy}
          >
            <Save />
          </button>

          <button
            type="button"
            className="icon-button is-danger"
            onClick={onDiscard}
            aria-label="放弃未保存修改"
            title={dirty ? "放弃未保存修改" : "当前没有需要放弃的修改"}
            disabled={!dirty || anyBusy}
          >
            <Undo2 />
          </button>

          <button
            type="button"
            className="icon-button"
            onClick={onBack}
            aria-label="返回工作台"
            title={
              dirty
                ? "请先保存或放弃修改后再返回工作台"
                : anyBusy
                  ? "当前有操作在执行，请稍后再试"
                  : "返回工作台"
            }
            disabled={dirty || anyBusy}
          >
            <ArrowLeft />
          </button>
        </div>
      </div>

      <article className="editor-paper" aria-label="编辑终稿">
        <div className="paper-content editor-field">
          <textarea
            className="editor-textarea"
            value={text}
            onChange={(event) => onChangeText(event.target.value)}
            spellCheck={false}
            aria-label="文档内容"
            placeholder="在此编辑文档内容…"
          />
        </div>
      </article>
    </div>
  );
});

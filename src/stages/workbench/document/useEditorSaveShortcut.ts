import { useEffect } from "react";

export function useEditorSaveShortcut({
  busy,
  dirty,
  onSave
}: {
  busy: boolean;
  dirty: boolean;
  onSave: () => void;
}) {
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      const key = event.key.toLowerCase();
      if (!(event.ctrlKey || event.metaKey) || key !== "s") return;
      event.preventDefault();
      if (!dirty || busy) return;
      onSave();
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [busy, dirty, onSave]);
}

import { useCallback, useRef, useState } from "react";
import type { NoticeState, NoticeTone } from "../lib/constants";

const AUTO_DISMISS_MS = 6000;

/**
 * 管理通知状态，提供自动消失定时器。
 */
export function useNotice() {
  const [notice, setNotice] = useState<NoticeState | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const dismissNotice = useCallback(() => {
    setNotice(null);
    if (timerRef.current) {
      clearTimeout(timerRef.current);
      timerRef.current = null;
    }
  }, []);

  const showNotice = useCallback(
    (tone: NoticeTone, message: string) => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      setNotice({ tone, message });
      timerRef.current = setTimeout(() => {
        setNotice(null);
        timerRef.current = null;
      }, AUTO_DISMISS_MS);
    },
    []
  );

  return { notice, showNotice, dismissNotice } as const;
}

import { useCallback, useRef, useState } from "react";
import type { NoticeState, NoticeTone } from "../lib/constants";

const AUTO_DISMISS_MS = 6000;

type NoticeOptions = {
  /**
   * 自动关闭时间（毫秒）。
   * - 省略：使用默认值
   * - null/<=0：常驻，直到手动关闭或被下一条提示覆盖
   */
  autoDismissMs?: number | null;
};

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
    (tone: NoticeTone, message: string, options?: NoticeOptions) => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
      const autoDismissMs = options?.autoDismissMs ?? AUTO_DISMISS_MS;
      setNotice({ tone, message, autoDismissMs });
      timerRef.current = null;

      if (autoDismissMs == null || autoDismissMs <= 0) {
        return;
      }

      timerRef.current = setTimeout(() => {
        setNotice(null);
        timerRef.current = null;
      }, autoDismissMs);
    },
    []
  );

  return { notice, showNotice, dismissNotice } as const;
}

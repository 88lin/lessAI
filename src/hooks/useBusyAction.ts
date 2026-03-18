import { useCallback, useRef, useState } from "react";

/**
 * 管理异步操作的 loading 状态，防止并发重复触发。
 */
export function useBusyAction() {
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const busyActionRef = useRef<string | null>(null);
  const pendingPromiseRef = useRef<Promise<unknown> | null>(null);

  const withBusy = useCallback(
    async <T,>(actionKey: string, operation: () => Promise<T>): Promise<T> => {
      if (busyActionRef.current) {
        if (busyActionRef.current === actionKey && pendingPromiseRef.current) {
          return pendingPromiseRef.current as Promise<T>;
        }

        throw new Error("已有操作在执行，请稍后再试。");
      }

      busyActionRef.current = actionKey;
      setBusyAction(actionKey);

      const promise = operation();
      pendingPromiseRef.current = promise as unknown as Promise<unknown>;

      try {
        return await promise;
      } finally {
        if (pendingPromiseRef.current === promise) {
          pendingPromiseRef.current = null;
          busyActionRef.current = null;
          setBusyAction((current) => (current === actionKey ? null : current));
        }
      }
    },
    []
  );

  const isBusy = useCallback(
    (actionKey: string) => busyAction === actionKey,
    [busyAction]
  );

  return { busyAction, withBusy, isBusy } as const;
}

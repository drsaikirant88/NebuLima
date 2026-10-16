import { useCallback, useRef, useState } from "react";
import type { IpcError } from "@/types/ipc";

export type AsyncActionStatus = "idle" | "pending" | "success" | "error";

export interface UseAsyncActionResult<Args extends unknown[], Result> {
  status: AsyncActionStatus;
  error: IpcError | null;
  result: Result | null;
  /** True while `run` is in flight, for immediate pending feedback. */
  isPending: boolean;
  run: (...args: Args) => Promise<Result | undefined>;
  reset: () => void;
}

/**
 * Wraps any IPC command invocation with pending/success/error state so every action
 * button (start/stop a profile, delete a container, prune volumes, ...) can show
 * immediate feedback and a clear outcome without re-implementing the same three
 * `useState` calls everywhere.
 */
export function useAsyncAction<Args extends unknown[], Result>(
  action: (...args: Args) => Promise<Result>
): UseAsyncActionResult<Args, Result> {
  const [status, setStatus] = useState<AsyncActionStatus>("idle");
  const [error, setError] = useState<IpcError | null>(null);
  const [result, setResult] = useState<Result | null>(null);
  // Guards against a stale response from an earlier call overwriting state after a
  // newer call has already started (e.g. rapid double-clicks on a start/stop toggle).
  const callId = useRef(0);

  const run = useCallback(
    async (...args: Args) => {
      const thisCallId = ++callId.current;
      setStatus("pending");
      setError(null);
      try {
        const value = await action(...args);
        if (callId.current === thisCallId) {
          setResult(value);
          setStatus("success");
        }
        return value;
      } catch (err) {
        if (callId.current === thisCallId) {
          setError(err as IpcError);
          setStatus("error");
        }
        return undefined;
      }
    },
    [action]
  );

  const reset = useCallback(() => {
    callId.current += 1;
    setStatus("idle");
    setError(null);
    setResult(null);
  }, []);

  return { status, error, result, isPending: status === "pending", run, reset };
}

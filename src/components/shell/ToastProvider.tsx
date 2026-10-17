import { Alert, Snackbar } from "@mui/material";
import type React from "react";
import { useCallback, useMemo, useState } from "react";
import { ToastContext } from "@/hooks/useToast";

type ToastSeverity = "success" | "error" | "info";

interface Toast {
  message: string;
  severity: ToastSeverity;
}

/**
 * App-wide snackbar surface: several flows (profile create/update, future delete
 * confirmations) only need a transient confirmation/error message, not a dialog that
 * blocks interaction — this is the single place that renders one.
 */
export const ToastProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [toast, setToast] = useState<Toast | null>(null);

  const showToast = useCallback((message: string, severity: ToastSeverity = "info") => {
    setToast({ message, severity });
  }, []);

  const value = useMemo(() => ({ showToast }), [showToast]);

  return (
    <ToastContext.Provider value={value}>
      {children}
      <Snackbar
        open={toast !== null}
        autoHideDuration={4000}
        onClose={() => setToast(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "center" }}
      >
        {toast ? (
          <Alert severity={toast.severity} onClose={() => setToast(null)} variant="filled">
            {toast.message}
          </Alert>
        ) : undefined}
      </Snackbar>
    </ToastContext.Provider>
  );
};

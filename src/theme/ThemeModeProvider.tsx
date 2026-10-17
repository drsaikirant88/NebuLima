import { CssBaseline } from "@mui/material";
import { ThemeProvider } from "@mui/material/styles";
import type React from "react";
import { createContext, useCallback, useContext, useEffect, useMemo, useState } from "react";
import { getSettings, updateSettings } from "@/services/ipc";
import type { ThemeMode } from "@/types/ipc";
import { buildTheme } from "./theme";

interface ThemeModeContextValue {
  mode: ThemeMode;
  toggleMode: () => void;
}

const ThemeModeContext = createContext<ThemeModeContextValue | null>(null);

// biome-ignore lint/style/useComponentExportOnlyModules: Provider + its own context-accessor hook is the standard React context pattern; splitting the hook out would separate it from the context it reads.
export function useThemeMode(): ThemeModeContextValue {
  const ctx = useContext(ThemeModeContext);
  if (!ctx) {
    throw new Error("useThemeMode must be used within a ThemeModeProvider");
  }
  return ctx;
}

export const ThemeModeProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [mode, setMode] = useState<ThemeMode>("dark");

  useEffect(() => {
    let cancelled = false;
    getSettings()
      .then((settings) => {
        if (!cancelled) setMode(settings.themeMode);
      })
      .catch(() => {
        // Settings not yet reachable (e.g. first run before the backend finishes
        // initializing) — fall back silently to the default dark mode.
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const toggleMode = useCallback(() => {
    setMode((prev) => {
      const next: ThemeMode = prev === "dark" ? "light" : "dark";
      updateSettings({ themeMode: next }).catch(() => {
        // Persisting the preference is best-effort; the UI already reflects the toggle.
      });
      return next;
    });
  }, []);

  const theme = useMemo(() => buildTheme(mode), [mode]);
  const contextValue = useMemo(() => ({ mode, toggleMode }), [mode, toggleMode]);

  return (
    <ThemeModeContext.Provider value={contextValue}>
      <ThemeProvider theme={theme}>
        <CssBaseline />
        {children}
      </ThemeProvider>
    </ThemeModeContext.Provider>
  );
};

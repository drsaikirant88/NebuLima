import AddIcon from "@mui/icons-material/Add";
import CloseIcon from "@mui/icons-material/Close";
import DarkModeIcon from "@mui/icons-material/DarkMode";
import HelpOutlineIcon from "@mui/icons-material/HelpOutlineOutlined";
import LightModeIcon from "@mui/icons-material/LightMode";
import RemoveIcon from "@mui/icons-material/Remove";
import { Box, IconButton, Link, Popover, Stack, Typography } from "@mui/material";
import { getName, getVersion } from "@tauri-apps/api/app";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-shell";
import type React from "react";
import { useState } from "react";
import { glassSurface } from "@/theme/glass";
import { useThemeMode } from "@/theme/ThemeModeProvider";

const TITLE_BAR_HEIGHT = 40;
const TRAFFIC_LIGHT_SIZE = 14;
const APP_GITHUB_URL = "https://github.com/drsaikirant88/NebuLima";

interface TrafficLightProps {
  color: string;
  label: string;
  icon: React.ReactNode;
  onClick: () => void;
}

/**
 * A single macOS-style "traffic light" button — a filled circle that only reveals its
 * glyph on hover, matching the native convention instead of showing a generic icon button.
 * Recent macOS doesn't relighten the circle itself on hover, it darkens the revealed glyph
 * instead — mirrored here via the glyph's `color`, not the circle's `backgroundColor`.
 */
const TrafficLight: React.FC<TrafficLightProps> = ({ color, label, icon, onClick }) => (
  <Box
    component="button"
    type="button"
    aria-label={label}
    onClick={onClick}
    sx={{
      width: TRAFFIC_LIGHT_SIZE,
      height: TRAFFIC_LIGHT_SIZE,
      borderRadius: "50%",
      border: "none",
      p: 0,
      display: "flex",
      alignItems: "center",
      justifyContent: "center",
      cursor: "default",
      backgroundColor: color,
      color: "rgba(0, 0, 0, 0.55)",
      "& svg": { fontSize: 9, opacity: 0, transition: "opacity 120ms ease" },
      "&:hover": { color: "rgba(0, 0, 0, 0.75)" },
      "&:hover svg": { opacity: 1 },
      "&:focus-visible": { outline: "2px solid currentColor", outlineOffset: 2 },
    }}
  >
    {icon}
  </Box>
);

/**
 * Custom window chrome: `tauri.conf.json` sets `decorations: false`, so the OS draws no
 * titlebar at all. Traffic lights sit on the left, matching macOS convention (close,
 * minimize, zoom left-to-right).
 */
export const TitleBar: React.FC = () => {
  const { mode, toggleMode } = useThemeMode();
  const [infoAnchor, setInfoAnchor] = useState<HTMLElement | null>(null);
  const [appInfo, setAppInfo] = useState<{ name: string; version: string } | null>(null);

  const handleMinimize = () => void getCurrentWindow().minimize();
  const handleToggleMaximize = () => void getCurrentWindow().toggleMaximize();
  const handleClose = () => void getCurrentWindow().close();

  // The native `data-tauri-drag-region` attribute only matches when the exact
  // mousedown target carries it, so clicks on the title text work but clicks on the
  // surrounding bar background don't always bubble correctly across platforms — call
  // `startDragging` explicitly instead, skipping any click that lands on a button.
  const handleBarMouseDown = (event: React.MouseEvent<HTMLElement>) => {
    if (event.button !== 0 || (event.target as HTMLElement).closest("button")) return;
    void getCurrentWindow().startDragging();
  };

  const handleBarDoubleClick = (event: React.MouseEvent<HTMLElement>) => {
    if ((event.target as HTMLElement).closest("button")) return;
    void getCurrentWindow().toggleMaximize();
  };

  const handleInfoClick = async (event: React.MouseEvent<HTMLElement>) => {
    setInfoAnchor(event.currentTarget);
    if (appInfo) return;
    try {
      const [name, version] = await Promise.all([getName(), getVersion()]);
      setAppInfo({ name, version });
    } catch {
      setAppInfo({ name: "NebuLima", version: "0.1.0" });
    }
  };

  return (
    <Box
      data-tauri-drag-region
      onMouseDown={handleBarMouseDown}
      onDoubleClick={handleBarDoubleClick}
      sx={{
        height: TITLE_BAR_HEIGHT,
        display: "flex",
        alignItems: "center",
        justifyContent: "space-between",
        px: 2,
        userSelect: "none",
        ...glassSurface(mode, 18),
      }}
    >
      <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
        <TrafficLight
          color="#EC6765"
          label="Close window"
          icon={<CloseIcon />}
          onClick={handleClose}
        />
        <TrafficLight
          color="#F2CA44"
          label="Minimize window"
          icon={<RemoveIcon />}
          onClick={handleMinimize}
        />
        <TrafficLight
          color="#65C466"
          label="Maximize or restore window"
          icon={<AddIcon />}
          onClick={handleToggleMaximize}
        />
      </Stack>

      <Typography
        data-tauri-drag-region
        variant="subtitle2"
        sx={{ fontWeight: 700, letterSpacing: 0.4, flexGrow: 1, textAlign: "center" }}
      >
        NebuLima
      </Typography>

      <Stack direction="row" spacing={0.5} sx={{ alignItems: "center" }}>
        <IconButton size="small" aria-label="About NebuLima" onClick={handleInfoClick}>
          <HelpOutlineIcon fontSize="small" />
        </IconButton>
        <IconButton
          size="small"
          aria-label={mode === "dark" ? "Switch to light mode" : "Switch to dark mode"}
          onClick={toggleMode}
        >
          {mode === "dark" ? <LightModeIcon fontSize="small" /> : <DarkModeIcon fontSize="small" />}
        </IconButton>
      </Stack>

      <Popover
        open={Boolean(infoAnchor)}
        anchorEl={infoAnchor}
        onClose={() => setInfoAnchor(null)}
        anchorOrigin={{ vertical: "bottom", horizontal: "right" }}
        transformOrigin={{ vertical: "top", horizontal: "right" }}
      >
        <Stack spacing={0.5} sx={{ p: 2, minWidth: 200, maxWidth: 240 }}>
          <Typography variant="subtitle1" sx={{ fontWeight: 700 }}>
            {appInfo?.name ?? "NebuLima"}
          </Typography>
          <Typography variant="body2" color="text.secondary">
            Version {appInfo?.version ?? "…"}
          </Typography>
          <Link
            component="button"
            type="button"
            variant="body2"
            onClick={() => void open(APP_GITHUB_URL)}
            sx={{ alignSelf: "flex-start" }}
          >
            Project page
          </Link>
          <Typography variant="caption" color="text.secondary" sx={{ pt: 0.5 }}>
            Built and validated against Colima v0.10.3, Docker v29.4.3, and kubectl v1.37.1.
            Provided as-is, with no warranty - use at your own risk; the maintainers aren't
            responsible for any issues or damages arising from its use.
          </Typography>
        </Stack>
      </Popover>
    </Box>
  );
};

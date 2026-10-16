import { Box, Button, Stack, Typography } from "@mui/material";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { useContainerLogs } from "@/hooks/useContainerLogs";
import { glassSurface } from "@/theme/glass";
import { PALETTE } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";

interface ContainerLogPanelProps {
  containerId: string;
  containerName: string;
  profileName: string;
}

const NEAR_BOTTOM_PX = 48;

/**
 * Live, scrollable log view: auto-follows new lines while the user is scrolled to the
 * bottom, and pauses auto-scroll (showing a "Jump to latest" button instead) the moment
 * they scroll up to read history. Switching `containerId` resets both the buffer and the
 * auto-scroll state via `useContainerLogs`.
 */
export const ContainerLogPanel: React.FC<ContainerLogPanelProps> = ({
  containerId,
  containerName,
  profileName,
}) => {
  const { mode } = useThemeMode();
  const { lines } = useContainerLogs(profileName, containerId);
  const [autoScroll, setAutoScroll] = useState(true);
  const scrollRef = useRef<HTMLDivElement>(null);

  // biome-ignore lint/correctness/useExhaustiveDependencies: containerId is a trigger-only dependency that resets auto-scroll on container switch without being read in the body; removing it would stop the reset from firing.
  useEffect(() => {
    setAutoScroll(true);
  }, [containerId]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: lines is a trigger-only dependency that re-scrolls on new log lines without being read in the body; removing it would stop auto-scroll-to-bottom.
  useEffect(() => {
    if (!autoScroll) return;
    const el = scrollRef.current;
    if (el) el.scrollTop = el.scrollHeight;
  }, [lines, autoScroll]);

  const handleScroll = () => {
    const el = scrollRef.current;
    if (!el) return;
    const distanceFromBottom = el.scrollHeight - el.scrollTop - el.clientHeight;
    setAutoScroll(distanceFromBottom <= NEAR_BOTTOM_PX);
  };

  return (
    <Box
      sx={{
        p: 1.5,
        borderRadius: 2,
        height: 280,
        display: "flex",
        flexDirection: "column",
        ...glassSurface(mode, 12),
      }}
    >
      <Stack direction="row" sx={{ justifyContent: "space-between", alignItems: "center", mb: 1 }}>
        <Typography variant="subtitle2">Logs — {containerName}</Typography>
        {!autoScroll && (
          <Button size="small" onClick={() => setAutoScroll(true)}>
            Jump to latest
          </Button>
        )}
      </Stack>
      <Box
        ref={scrollRef}
        onScroll={handleScroll}
        sx={{
          flex: 1,
          overflow: "auto",
          fontFamily: "monospace",
          fontSize: 12,
          whiteSpace: "pre-wrap",
          overflowWrap: "anywhere",
        }}
      >
        {lines.map((line) => (
          <Box
            key={line.seq}
            sx={{ color: line.stream === "stderr" ? PALETTE.mutedMauve : "inherit" }}
          >
            {line.line}
          </Box>
        ))}
      </Box>
    </Box>
  );
};

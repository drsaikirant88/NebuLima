import { Box, Typography } from "@mui/material";
import type React from "react";
import { Legend, Line, LineChart, ResponsiveContainer, Tooltip, XAxis, YAxis } from "recharts";
import { useContainerStats } from "@/hooks/useContainerStats";
import { glassSurface } from "@/theme/glass";
import { PALETTE } from "@/theme/palette";
import { useThemeMode } from "@/theme/ThemeModeProvider";

interface ResourceGraphProps {
  containerId: string | null;
}

function formatTime(iso: string): string {
  const date = new Date(iso);
  return Number.isNaN(date.getTime()) ? "" : date.toLocaleTimeString([], { hour12: false });
}

/**
 * Live CPU%/memory trend line over `useContainerStats`' bounded one-minute window. Both
 * series reuse the existing palette colors; no new hue.
 */
export const ResourceGraph: React.FC<ResourceGraphProps> = ({ containerId }) => {
  const { mode } = useThemeMode();
  const { history } = useContainerStats(containerId);

  const data = history.map((sample) => ({
    time: formatTime(sample.sampledAt),
    cpuPercent: Math.round(sample.cpuPercent * 10) / 10,
    memoryMiB: Math.round(sample.memoryBytes / 1024 / 1024),
  }));

  if (!containerId || data.length === 0) {
    return (
      <Box sx={{ p: 2, borderRadius: 2, ...glassSurface(mode, 12) }}>
        <Typography variant="body2" color="text.secondary">
          No resource data yet.
        </Typography>
      </Box>
    );
  }

  return (
    <Box sx={{ p: 1.5, borderRadius: 2, height: 220, ...glassSurface(mode, 12) }}>
      <Typography variant="subtitle2" sx={{ mb: 1 }}>
        CPU % / Memory (MiB)
      </Typography>
      <ResponsiveContainer width="100%" height="85%">
        <LineChart data={data} margin={{ top: 4, right: 4, bottom: 0, left: 4 }}>
          <XAxis dataKey="time" hide />
          <YAxis
            yAxisId="cpu"
            domain={[0, "auto"]}
            width={36}
            tick={{ fontSize: 10, fill: PALETTE.warmSand }}
            axisLine={{ stroke: PALETTE.warmSand }}
            tickLine={false}
          />
          <YAxis
            yAxisId="mem"
            orientation="right"
            domain={[0, "auto"]}
            width={48}
            tick={{ fontSize: 10, fill: PALETTE.mutedMauve }}
            axisLine={{ stroke: PALETTE.mutedMauve }}
            tickLine={false}
          />
          <Tooltip />
          <Legend verticalAlign="top" height={24} wrapperStyle={{ fontSize: 12 }} />
          <Line
            yAxisId="cpu"
            type="monotone"
            dataKey="cpuPercent"
            name="CPU %"
            stroke={PALETTE.warmSand}
            dot={false}
          />
          <Line
            yAxisId="mem"
            type="monotone"
            dataKey="memoryMiB"
            name="Memory MiB"
            stroke={PALETTE.mutedMauve}
            dot={false}
          />
        </LineChart>
      </ResponsiveContainer>
    </Box>
  );
};

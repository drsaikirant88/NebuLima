import DnsIcon from "@mui/icons-material/Dns";
import LayersIcon from "@mui/icons-material/Layers";
import StorageIcon from "@mui/icons-material/Storage";
import ViewInArIcon from "@mui/icons-material/ViewInAr";
import { Box, List, ListItemButton, ListItemIcon, ListItemText } from "@mui/material";
import type React from "react";
import { glassSurface } from "@/theme/glass";
import { useThemeMode } from "@/theme/ThemeModeProvider";
import { useRovingTabIndex } from "./useRovingTabIndex";

export type SidebarView = "profiles" | "containers" | "images" | "volumes";

const NAV_ITEMS: { view: SidebarView; label: string; icon: React.ReactNode }[] = [
  { view: "profiles", label: "Profiles", icon: <DnsIcon fontSize="small" /> },
  { view: "containers", label: "Containers", icon: <ViewInArIcon fontSize="small" /> },
  { view: "images", label: "Images", icon: <LayersIcon fontSize="small" /> },
  { view: "volumes", label: "Volumes", icon: <StorageIcon fontSize="small" /> },
];

interface SidebarProps {
  activeView: SidebarView;
  onSelectView: (view: SidebarView) => void;
}

/**
 * Floating left-hand navigation frame. Arrow-key roving focus lets keyboard users move
 * between entries without burning extra Tab stops.
 */
export const Sidebar: React.FC<SidebarProps> = ({ activeView, onSelectView }) => {
  const { mode } = useThemeMode();
  const { containerRef, handleKeyDown } = useRovingTabIndex<HTMLUListElement>(NAV_ITEMS.length);

  return (
    <Box
      component="nav"
      aria-label="Primary"
      sx={{
        width: 220,
        p: 1.5,
        borderRadius: 3,
        ...glassSurface(mode, 24),
      }}
    >
      <List
        ref={containerRef}
        onKeyDown={handleKeyDown}
        role="listbox"
        aria-label="Views"
        disablePadding
      >
        {NAV_ITEMS.map((item) => (
          <ListItemButton
            key={item.view}
            data-roving-item="true"
            role="option"
            aria-selected={activeView === item.view}
            selected={activeView === item.view}
            onClick={() => onSelectView(item.view)}
            sx={{ mb: 0.5 }}
          >
            <ListItemIcon sx={{ minWidth: 36 }}>{item.icon}</ListItemIcon>
            <ListItemText primary={item.label} />
          </ListItemButton>
        ))}
      </List>
    </Box>
  );
};

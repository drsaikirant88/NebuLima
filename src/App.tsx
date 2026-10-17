import { Box } from "@mui/material";
import type React from "react";
import { useState } from "react";
import { ContainersView } from "@/components/containers/ContainersView";
import { ImagesView } from "@/components/images/ImagesView";
import { ProfilesView } from "@/components/profiles/ProfilesView";
import { Sidebar, type SidebarView } from "@/components/shell/Sidebar";
import { TitleBar } from "@/components/shell/TitleBar";
import { ToastProvider } from "@/components/shell/ToastProvider";
import { VolumesView } from "@/components/volumes/VolumesView";
import { windowBackdropGradient } from "@/theme/glass";
import { ThemeModeProvider, useThemeMode } from "@/theme/ThemeModeProvider";

const VIEW_LABELS: Record<SidebarView, string> = {
  profiles: "Profiles",
  containers: "Containers",
  images: "Images",
  volumes: "Volumes",
};

const AppShell: React.FC = () => {
  const { mode } = useThemeMode();
  const [activeView, setActiveView] = useState<SidebarView>("profiles");
  const [resourceProfileFilter, setResourceProfileFilter] = useState<string | null>(null);

  const handleViewResources = (profileName: string) => {
    setResourceProfileFilter(profileName);
    setActiveView("containers");
  };

  return (
    <Box
      sx={{
        height: "100%",
        display: "flex",
        flexDirection: "column",
        borderRadius: "12px",
        overflow: "hidden",
        ...windowBackdropGradient(mode),
      }}
    >
      <TitleBar />
      <Box sx={{ flex: 1, display: "flex", gap: 2, p: 2, overflow: "hidden" }}>
        <Sidebar activeView={activeView} onSelectView={setActiveView} />
        <Box
          component="main"
          sx={{ flex: 1, minWidth: 0, overflowX: "hidden", overflowY: "auto", pt: 1.5 }}
          role="region"
          aria-label={VIEW_LABELS[activeView]}
        >
          {activeView === "profiles" && <ProfilesView onViewResources={handleViewResources} />}
          {activeView === "containers" && (
            <ContainersView initialProfileFilter={resourceProfileFilter} />
          )}
          {activeView === "images" && <ImagesView initialProfileFilter={resourceProfileFilter} />}
          {activeView === "volumes" && <VolumesView initialProfileFilter={resourceProfileFilter} />}
        </Box>
      </Box>
    </Box>
  );
};

const App: React.FC = () => (
  <ThemeModeProvider>
    <ToastProvider>
      <AppShell />
    </ToastProvider>
  </ThemeModeProvider>
);

export default App;

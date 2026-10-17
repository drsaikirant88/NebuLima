import { Box, Stack, Typography } from "@mui/material";
import type React from "react";
import { useState } from "react";
import { useContainers } from "@/hooks/useContainers";
import { useResourceProfileFilter } from "@/hooks/useResourceProfileFilter";
import { deleteContainer } from "@/services/ipc";
import { PALETTE } from "@/theme/palette";
import type { Container, IpcError } from "@/types/ipc";
import { ResourceProfileFilter } from "../shared/ResourceProfileFilter";
import { type ResizableColumn, ContainerRow } from "./ContainerRow";
import { DeleteContainerDialog } from "./DeleteContainerDialog";

const DEFAULT_COLUMN_WIDTHS: Record<ResizableColumn, number> = { name: 160, image: 160 };

/**
 * Containers view: a live list where each row can expand in place to show its own
 * resource graph and live log tail directly under that row, one at a time. Delete always
 * shows the confirmation dialog unconditionally — unlike profiles/images/volumes, there is
 * no speculative unconfirmed call first. The "Profile" filter scopes the list to one profile
 * or every running profile; each row's own `profileName` drives its start/stop/delete/logs
 * calls, since "all profiles" mode mixes rows from different profiles.
 */
interface ContainersViewProps {
  /** Pre-selects the "Profile" filter — set when navigating here from the Profiles
   * view's "view resources" action rather than the sidebar. */
  initialProfileFilter?: string | null;
}

export const ContainersView: React.FC<ContainersViewProps> = ({ initialProfileFilter = null }) => {
  const { profiles, profileName, setProfileName } = useResourceProfileFilter(initialProfileFilter);
  const { containers, isLoading, error } = useContainers(profileName);
  const [expandedId, setExpandedId] = useState<string | null>(null);
  const [columnWidths, setColumnWidths] = useState(DEFAULT_COLUMN_WIDTHS);
  const [deleteTarget, setDeleteTarget] = useState<Container | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<IpcError | null>(null);

  const handleResizeColumn = (column: ResizableColumn, width: number) =>
    setColumnWidths((prev) => ({ ...prev, [column]: width }));

  const handleToggleExpanded = (container: Container) =>
    setExpandedId((prev) => (prev === container.id ? null : container.id));

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteContainer(deleteTarget.profileName, deleteTarget.id, true);
      if (expandedId === deleteTarget.id) setExpandedId(null);
      setDeleteTarget(null);
    } catch (err) {
      setDeleteError(err as IpcError);
    } finally {
      setIsDeleting(false);
    }
  };

  return (
    <Box>
      <Stack direction="row" spacing={2} sx={{ alignItems: "center", mb: 2 }}>
        <Typography variant="h6">Containers</Typography>
        <Box sx={{ flex: 1 }} />
        <ResourceProfileFilter profiles={profiles} value={profileName} onChange={setProfileName} />
      </Stack>

      {deleteError && (
        <Typography variant="body2" sx={{ color: PALETTE.mutedMauve, mb: 1 }}>
          {deleteError.message}
        </Typography>
      )}

      {!isLoading && error && (
        <Typography variant="body2" color="text.secondary">
          {error.message}
        </Typography>
      )}

      {!isLoading && !error && containers.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          No containers found for the selected profile.
        </Typography>
      )}

      <Stack spacing={1} sx={{ mb: 2 }}>
        {containers.map((container) => (
          <ContainerRow
            key={container.id}
            container={container}
            isExpanded={container.id === expandedId}
            onToggleExpanded={handleToggleExpanded}
            onRequestDelete={setDeleteTarget}
            columnWidths={columnWidths}
            onResizeColumn={handleResizeColumn}
          />
        ))}
      </Stack>

      <DeleteContainerDialog
        container={deleteTarget}
        isSubmitting={isDeleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={confirmDelete}
      />
    </Box>
  );
};

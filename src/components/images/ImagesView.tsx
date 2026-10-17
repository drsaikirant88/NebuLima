import { Box, Stack, Typography } from "@mui/material";
import type React from "react";
import { useState } from "react";
import { useImages } from "@/hooks/useImages";
import { useResourceProfileFilter } from "@/hooks/useResourceProfileFilter";
import { deleteImage, inspectImage } from "@/services/ipc";
import { PALETTE } from "@/theme/palette";
import type { Image, ImageInspectDetail, IpcError } from "@/types/ipc";
import { ResourceProfileFilter } from "../shared/ResourceProfileFilter";
import { DeleteImageDialog } from "./DeleteImageDialog";
import { ImageInspectDialog } from "./ImageInspectDialog";
import { ImageRow } from "./ImageRow";

/**
 * Images view: a live list plus an inspect dialog and a delete-confirmation flow.
 * `delete_image` is always attempted unconfirmed first; the confirmation dialog only
 * appears when the backend reports `CONFIRMATION_REQUIRED` (the image is referenced by a
 * current container). The "Profile" filter scopes the list to one profile or every running
 * profile; each row's own `profileName` (not the filter) drives its inspect/delete calls,
 * since "all profiles" mode mixes rows from different profiles.
 */
interface ImagesViewProps {
  /** Pre-selects the "Profile" filter — set when navigating here from the Profiles
   * view's "view resources" action rather than the sidebar. */
  initialProfileFilter?: string | null;
}

export const ImagesView: React.FC<ImagesViewProps> = ({ initialProfileFilter = null }) => {
  const { profiles, profileName, setProfileName } = useResourceProfileFilter(initialProfileFilter);
  const { images, isLoading, error } = useImages(profileName);
  const [referenceWidth, setReferenceWidth] = useState(220);

  const [inspectTarget, setInspectTarget] = useState<Image | null>(null);
  const [inspectDetail, setInspectDetail] = useState<ImageInspectDetail | null>(null);
  const [isInspecting, setIsInspecting] = useState(false);
  const [inspectError, setInspectError] = useState<IpcError | null>(null);

  const [deleteTarget, setDeleteTarget] = useState<Image | null>(null);
  const [isDeleting, setIsDeleting] = useState(false);
  const [deleteError, setDeleteError] = useState<IpcError | null>(null);

  const handleInspect = async (image: Image) => {
    setInspectTarget(image);
    setInspectDetail(null);
    setInspectError(null);
    setIsInspecting(true);
    try {
      const detail = await inspectImage(image.profileName, image.id);
      setInspectDetail(detail);
    } catch (err) {
      setInspectError(err as IpcError);
    } finally {
      setIsInspecting(false);
    }
  };

  const requestDelete = async (image: Image) => {
    setDeleteError(null);
    try {
      await deleteImage(image.profileName, image.id, false);
    } catch (err) {
      const ipcError = err as IpcError;
      if (ipcError.code === "CONFIRMATION_REQUIRED") {
        setDeleteTarget(image);
      } else {
        setDeleteError(ipcError);
      }
    }
  };

  const confirmDelete = async () => {
    if (!deleteTarget) return;
    setIsDeleting(true);
    setDeleteError(null);
    try {
      await deleteImage(deleteTarget.profileName, deleteTarget.id, true);
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
        <Typography variant="h6">Images</Typography>
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

      {!isLoading && !error && images.length === 0 && (
        <Typography variant="body2" color="text.secondary">
          No images found for the selected profile.
        </Typography>
      )}

      <Stack spacing={1}>
        {images.map((image) => (
          <ImageRow
            key={image.id}
            image={image}
            referenceWidth={referenceWidth}
            onResizeReference={setReferenceWidth}
            onInspect={handleInspect}
            onRequestDelete={requestDelete}
          />
        ))}
      </Stack>

      <ImageInspectDialog
        open={inspectTarget !== null}
        title={inspectTarget?.repository ?? inspectTarget?.id ?? ""}
        detail={inspectDetail}
        isLoading={isInspecting}
        errorMessage={inspectError?.message}
        onClose={() => setInspectTarget(null)}
      />

      <DeleteImageDialog
        image={deleteTarget}
        isSubmitting={isDeleting}
        onCancel={() => setDeleteTarget(null)}
        onConfirm={confirmDelete}
      />
    </Box>
  );
};

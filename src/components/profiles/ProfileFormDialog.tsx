import AddIcon from "@mui/icons-material/Add";
import DeleteIcon from "@mui/icons-material/Delete";
import {
  Button,
  Checkbox,
  Dialog,
  DialogActions,
  DialogContent,
  DialogTitle,
  FormControlLabel,
  IconButton,
  MenuItem,
  Stack,
  Switch,
  TextField,
  Typography,
} from "@mui/material";
import { open as openFileDialog } from "@tauri-apps/plugin-dialog";
import type React from "react";
import { useEffect, useRef, useState } from "react";
import { checkKubectlAvailable } from "@/services/ipc";
import { PALETTE } from "@/theme/palette";
import type { CaCertMode, ColimaProfile, ContainerRuntime, Mount } from "@/types/ipc";
import { PROFILE_NAME_PATTERN, type ProfileFormValue, toFormValue } from "./profileFormValue";

interface ProfileFormDialogProps {
  open: boolean;
  /** `null` means "create"; a profile means "edit" (name is then immutable). */
  profile: ColimaProfile | null;
  isSubmitting: boolean;
  errorMessage?: string | null;
  onCancel: () => void;
  onSubmit: (value: ProfileFormValue) => void;
}

/**
 * Shared create/edit form. Resource fields (CPU/RAM/disk/runtime/Kubernetes/network/
 * mounts) are locked whenever the profile being edited isn't `stopped` — Colima can only
 * apply configuration changes to a stopped VM (`update_profile`'s `PROFILE_NOT_STOPPED`).
 */
export const ProfileFormDialog: React.FC<ProfileFormDialogProps> = ({
  open,
  profile,
  isSubmitting,
  errorMessage,
  onCancel,
  onSubmit,
}) => {
  const [value, setValue] = useState<ProfileFormValue>(() => toFormValue(profile));
  const isEdit = profile !== null;
  const resourceFieldsLocked = isEdit && profile.state !== "stopped";
  const [kubectlMissing, setKubectlMissing] = useState(false);

  // Mount objects carry no unique id of their own, and removeMount can delete from the
  // middle of the array — a stable, position-independent id per row keeps React list keys
  // valid even after a mount in the middle is removed and later ones shift up.
  const nextMountId = useRef(0);
  const [mountIds, setMountIds] = useState<number[]>(() =>
    value.mounts.map(() => nextMountId.current++)
  );

  useEffect(() => {
    if (open) {
      const next = toFormValue(profile);
      setValue(next);
      setMountIds(next.mounts.map(() => nextMountId.current++));
      setKubectlMissing(false);
    }
  }, [open, profile]);

  const handleKubernetesToggle = async (checked: boolean) => {
    if (checked) {
      const available = await checkKubectlAvailable();
      if (!available) {
        setKubectlMissing(true);
        return;
      }
    }
    setKubectlMissing(false);
    setValue((prev) => ({ ...prev, kubernetesEnabled: checked }));
  };

  const updateMount = (index: number, patch: Partial<Mount>) => {
    setValue((prev) => ({
      ...prev,
      mounts: prev.mounts.map((m, i) => (i === index ? { ...m, ...patch } : m)),
    }));
  };

  const addMount = () => {
    setValue((prev) => ({
      ...prev,
      mounts: [...prev.mounts, { hostPath: "", guestPath: "", writable: false }],
    }));
    setMountIds((prev) => [...prev, nextMountId.current++]);
  };

  const removeMount = (index: number) => {
    setValue((prev) => ({ ...prev, mounts: prev.mounts.filter((_, i) => i !== index) }));
    setMountIds((prev) => prev.filter((_, i) => i !== index));
  };

  const handleCaCertModeChange = (mode: CaCertMode) => {
    setValue((prev) => ({
      ...prev,
      caCert: mode === "file" ? { mode, certPath: prev.caCert.certPath } : { mode },
    }));
  };

  const chooseCertFile = async () => {
    const path = await openFileDialog({ multiple: false });
    if (typeof path === "string") {
      setValue((prev) => ({ ...prev, caCert: { mode: "file", certPath: path } }));
    }
  };

  const trimmedName = value.name.trim();
  const nameIsValid = isEdit || PROFILE_NAME_PATTERN.test(trimmedName);
  const canSubmit = trimmedName.length > 0 && nameIsValid && !isSubmitting;

  return (
    <Dialog open={open} onClose={onCancel} maxWidth="sm" fullWidth>
      <DialogTitle>{isEdit ? `Edit profile "${profile.name}"` : "New profile"}</DialogTitle>
      <DialogContent>
        <Stack spacing={2} sx={{ mt: 1 }}>
          {errorMessage && (
            <Typography variant="body2" sx={{ color: PALETTE.mutedMauve }}>
              {errorMessage}
            </Typography>
          )}
          <TextField
            label="Name"
            value={value.name}
            disabled={isEdit}
            // Colima itself lowercases profile names (constitution-mandated single source of
            // truth: src-tauri/src/commands/profiles.rs's `normalize_profile_name`) — matching
            // that live here means what's shown while typing is exactly what gets created,
            // instead of silently diverging the moment the name is submitted.
            onChange={(e) => setValue((prev) => ({ ...prev, name: e.target.value.toLowerCase() }))}
            autoFocus={!isEdit}
            required
            error={!isEdit && trimmedName.length > 0 && !nameIsValid}
            helperText={
              !isEdit && trimmedName.length > 0 && !nameIsValid
                ? "Must start with a letter or digit and contain only letters, digits, '-', '_', or '.'"
                : undefined
            }
          />

          {resourceFieldsLocked && (
            <Typography variant="caption" color="text.secondary">
              Stop this profile to change its resources, runtime, or network settings.
            </Typography>
          )}

          <Stack direction="row" spacing={2}>
            <TextField
              label="CPUs"
              type="number"
              value={value.cpu}
              disabled={resourceFieldsLocked}
              onChange={(e) => setValue((prev) => ({ ...prev, cpu: Number(e.target.value) }))}
              slotProps={{ htmlInput: { min: 1 } }}
              fullWidth
            />
            <TextField
              label="Memory (GiB)"
              type="number"
              value={value.memoryGiB}
              disabled={resourceFieldsLocked}
              onChange={(e) => setValue((prev) => ({ ...prev, memoryGiB: Number(e.target.value) }))}
              slotProps={{ htmlInput: { min: 1 } }}
              fullWidth
            />
            <TextField
              label="Disk (GiB)"
              type="number"
              value={value.diskGiB}
              disabled={resourceFieldsLocked}
              onChange={(e) => setValue((prev) => ({ ...prev, diskGiB: Number(e.target.value) }))}
              slotProps={{ htmlInput: { min: 1 } }}
              fullWidth
            />
          </Stack>

          <TextField
            select
            label="Runtime"
            value={value.runtime}
            disabled={resourceFieldsLocked}
            onChange={(e) =>
              setValue((prev) => ({ ...prev, runtime: e.target.value as ContainerRuntime }))
            }
          >
            <MenuItem value="docker">docker</MenuItem>
            <MenuItem value="containerd">containerd</MenuItem>
          </TextField>

          <FormControlLabel
            control={
              <Switch
                checked={value.kubernetesEnabled}
                disabled={resourceFieldsLocked}
                onChange={(e) => handleKubernetesToggle(e.target.checked)}
              />
            }
            label="Kubernetes"
          />
          {kubectlMissing && (
            <Typography variant="caption" sx={{ color: PALETTE.mutedMauve }}>
              kubectl was not found on PATH. Install kubectl and try again to enable Kubernetes for
              this profile.
            </Typography>
          )}

          {!isEdit && (
            <FormControlLabel
              control={
                <Checkbox
                  checked={value.keepActive}
                  onChange={(e) => setValue((prev) => ({ ...prev, keepActive: e.target.checked }))}
                />
              }
              label="Keep profile active after creation"
            />
          )}

          <Stack direction="row" spacing={2}>
            <TextField
              label="DNS servers (comma-separated)"
              value={value.dns}
              disabled={resourceFieldsLocked}
              onChange={(e) => setValue((prev) => ({ ...prev, dns: e.target.value }))}
              fullWidth
            />
            <TextField
              label="Static IP address"
              value={value.ipAddress}
              disabled={resourceFieldsLocked}
              onChange={(e) => setValue((prev) => ({ ...prev, ipAddress: e.target.value }))}
              fullWidth
            />
          </Stack>

          <Stack spacing={1}>
            <Typography variant="subtitle2">Mounts</Typography>
            {value.mounts.map((mount, index) => (
              <Stack
                direction="row"
                spacing={1}
                sx={{ alignItems: "center" }}
                key={mountIds[index]}
              >
                <TextField
                  label="Host path"
                  size="small"
                  value={mount.hostPath}
                  disabled={resourceFieldsLocked}
                  onChange={(e) => updateMount(index, { hostPath: e.target.value })}
                  fullWidth
                />
                <TextField
                  label="Guest path"
                  size="small"
                  value={mount.guestPath}
                  disabled={resourceFieldsLocked}
                  onChange={(e) => updateMount(index, { guestPath: e.target.value })}
                  fullWidth
                />
                <FormControlLabel
                  control={
                    <Switch
                      size="small"
                      checked={mount.writable}
                      disabled={resourceFieldsLocked}
                      onChange={(e) => updateMount(index, { writable: e.target.checked })}
                    />
                  }
                  label="Writable"
                />
                <IconButton
                  aria-label={`Remove mount ${index + 1}`}
                  disabled={resourceFieldsLocked}
                  onClick={() => removeMount(index)}
                >
                  <DeleteIcon fontSize="small" />
                </IconButton>
              </Stack>
            ))}
            <Button
              startIcon={<AddIcon />}
              disabled={resourceFieldsLocked}
              onClick={addMount}
              sx={{ alignSelf: "flex-start" }}
            >
              Add mount
            </Button>
          </Stack>

          <Stack spacing={1}>
            <Typography variant="subtitle2">CA Cert</Typography>
            <Stack direction="row" spacing={1} sx={{ alignItems: "center" }}>
              <TextField
                select
                label="Trust a CA inside the VM"
                size="small"
                value={value.caCert.mode}
                disabled={resourceFieldsLocked}
                onChange={(e) => handleCaCertModeChange(e.target.value as CaCertMode)}
                fullWidth
              >
                <MenuItem value="none">None</MenuItem>
                <MenuItem value="file">Select a file…</MenuItem>
                <MenuItem value="systemBundle">Bundle system certs</MenuItem>
              </TextField>
              {value.caCert.mode === "file" && (
                <Button disabled={resourceFieldsLocked} onClick={chooseCertFile}>
                  Choose file
                </Button>
              )}
            </Stack>
            {value.caCert.mode === "file" && value.caCert.certPath && (
              <Typography variant="caption" color="text.secondary">
                {value.caCert.certPath}
              </Typography>
            )}
          </Stack>
        </Stack>
      </DialogContent>
      <DialogActions>
        <Button onClick={onCancel}>Cancel</Button>
        <Button variant="contained" disabled={!canSubmit} onClick={() => onSubmit(value)}>
          {isEdit ? "Save" : "Create"}
        </Button>
      </DialogActions>
    </Dialog>
  );
};

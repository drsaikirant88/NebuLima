import { TextField } from "@mui/material";
import type React from "react";
import type { ColimaProfile } from "@/types/ipc";

interface ResourceProfileFilterProps {
  profiles: ColimaProfile[];
  value: string | null;
  onChange: (profileName: string | null) => void;
}

/**
 * The "Profile" filter shared by the Containers/Images/Volumes views. Renders as a
 * native `<select>` (`slotProps.select.native`) rather than MUI's listbox-based `Select` so it
 * stays a plain form control under test — `fireEvent.change` and `toHaveValue` need an actual
 * `<select>` element, not MUI's div-based combobox.
 */
export const ResourceProfileFilter: React.FC<ResourceProfileFilterProps> = ({
  profiles,
  value,
  onChange,
}) => (
  <TextField
    select
    size="small"
    label="Profile"
    value={value ?? ""}
    onChange={(event) => onChange(event.target.value === "" ? null : event.target.value)}
    slotProps={{ select: { native: true }, inputLabel: { shrink: true } }}
    sx={{ minWidth: 180 }}
  >
    <option value="">All profiles</option>
    {profiles.map((profile) => (
      <option key={profile.name} value={profile.name}>
        {profile.name}
      </option>
    ))}
  </TextField>
);

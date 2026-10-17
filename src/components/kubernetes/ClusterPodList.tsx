import ExpandMoreIcon from "@mui/icons-material/ExpandMore";
import {
  Accordion,
  AccordionDetails,
  AccordionSummary,
  Chip,
  Stack,
  Typography,
} from "@mui/material";
import type React from "react";
import { useCallback, useState } from "react";
import { listClusterPods } from "@/services/ipc";
import type { ClusterPod, IpcError } from "@/types/ipc";

interface ClusterPodListProps {
  profileName: string;
  isClusterRunning: boolean;
}

/**
 * Expandable pod detail section. There is no live event for pods, so this queries
 * `list_cluster_pods` fresh each time it's expanded rather than keeping a subscription
 * live. Resolves to either the pod rows or an explicit "No pods." caption, never leaving
 * `isLoading` as the last word.
 */
export const ClusterPodList: React.FC<ClusterPodListProps> = ({
  profileName,
  isClusterRunning,
}) => {
  const [pods, setPods] = useState<ClusterPod[] | null>(null);
  const [error, setError] = useState<IpcError | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const loadPods = useCallback(async () => {
    setIsLoading(true);
    setError(null);
    try {
      setPods(await listClusterPods(profileName));
    } catch (err) {
      setError(err as IpcError);
      setPods(null);
    } finally {
      setIsLoading(false);
    }
  }, [profileName]);

  const handleChange = (_event: React.SyntheticEvent, isExpanded: boolean) => {
    if (isExpanded) void loadPods();
  };

  return (
    <Accordion disabled={!isClusterRunning} onChange={handleChange} disableGutters>
      <AccordionSummary expandIcon={<ExpandMoreIcon />}>
        <Typography variant="body2">Pods</Typography>
      </AccordionSummary>
      <AccordionDetails>
        {error ? (
          <Typography variant="caption">{error.message}</Typography>
        ) : isLoading ? (
          <Typography variant="caption">Loading pods…</Typography>
        ) : !pods || pods.length === 0 ? (
          <Typography variant="caption">No pods.</Typography>
        ) : (
          <Stack spacing={0.5}>
            {pods.map((pod) => (
              <Stack
                key={`${pod.namespace}/${pod.name}`}
                direction="row"
                spacing={1}
                sx={{ alignItems: "center" }}
              >
                <Typography variant="body2">
                  {pod.namespace}/{pod.name}
                </Typography>
                <Chip size="small" label={pod.phase} variant="outlined" />
              </Stack>
            ))}
          </Stack>
        )}
      </AccordionDetails>
    </Accordion>
  );
};

# Troubleshooting

| **Issue** | **Solution** |
| --- | --- |
| **"Missing required dependencies on PATH: ..." error dialog on launch.** | NebuLima's pre-flight check couldn't find `colima` and/or `docker` on `PATH`. Install whichever tool(s) are named in the message (see [Installation](../README.md#installation)) and relaunch the app — the main window won't appear until this check passes |
| **A Colima profile won't start, or gets stuck in `error` state.** | Check the profile's `errorReason` shown in the Profiles view, and try starting the same profile directly from a terminal (`colima start <profile-name>`) to see Colima's own diagnostic output. Common causes are insufficient host resources (CPU/RAM/disk requested exceeds what's available) or a conflicting VM already using the same resources |
| **The app can't find a Docker socket for a running profile.** | NebuLima only ever connects to `~/.colima/<profile-name>/docker.sock` — the exact path Colima itself creates for that profile. If a profile shows as `running` in Colima but NebuLima can't reach it, confirm that socket file exists (`ls ~/.colima/<profile-name>/`) and that the profile's container runtime is set to `docker` (NebuLima does not connect to `containerd`-runtime profiles) |
| **Enabling Kubernetes on a profile is blocked with a "kubectl was not found" message.** | NebuLima checks for `kubectl` on `PATH` before letting you turn the Kubernetes toggle on in the create/edit form, since Colima's embedded cluster is managed through `kubectl` once it's running. Install `kubectl` and try the toggle again |


**Still stuck?**
NebuLima writes a rotating diagnostic log via `tracing`, available through the app's `get_diagnostic_log_path` command, or directly on disk at:

- **macOS**: `~/Library/Logs/com.nebulima.app/`
- **Ubuntu**: `~/.local/share/com.nebulima.app/logs/` (or `$XDG_DATA_HOME/com.nebulima.app/logs/` if set)

Attach the relevant log file when [reporting an issue](https://github.com/drsaikirant88/NebuLima/issues).

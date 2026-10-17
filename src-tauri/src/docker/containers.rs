//! Container models and operations against the active profile's Docker engine.
//! `Container`/`ContainerPort`/`ResourceUsageSample`/`ContainerLatestStats` mirror
//! `src/types/ipc.ts` exactly. List/start/stop/remove and the per-profile stats/log
//! streaming tasks take an already-connected `&Docker`/`Arc<Docker>` rather than a socket
//! path, so the one place that ever turns a path into a connection is `connect()` below,
//! built on the validated socket from `docker/socket.rs`.
use crate::docker::socket::resolve_for_connection;
use crate::IpcError;
use bollard::models::{
    ContainerStatsResponse, ContainerSummary, ContainerSummaryStateEnum, PortSummary,
    PortSummaryTypeEnum,
};
use bollard::query_parameters::{
    ListContainersOptions, LogsOptions, RemoveContainerOptions, StatsOptions, StopContainerOptions,
};
use bollard::Docker;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;

const CONNECT_TIMEOUT_SECS: u64 = 10;
const STATS_POLL_INTERVAL: Duration = Duration::from_secs(1);

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ContainerStatus {
    Created,
    Running,
    Paused,
    Stopped,
    Exited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PortProtocol {
    Tcp,
    Udp,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerPort {
    pub container_port: u16,
    pub host_port: Option<u16>,
    pub protocol: PortProtocol,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceUsageSample {
    pub container_id: String,
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub sampled_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerLatestStats {
    pub cpu_percent: f64,
    pub memory_bytes: u64,
    pub memory_limit_bytes: u64,
    pub sampled_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Container {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: ContainerStatus,
    pub uptime_seconds: Option<i64>,
    pub ports: Vec<ContainerPort>,
    pub latest_stats: Option<ContainerLatestStats>,
    pub profile_name: String,
}

/// Formats a Unix timestamp as an RFC-3339-shaped UTC string (e.g. `2026-10-12T19:40:00.123Z`)
/// without a date/time crate dependency — every timestamp this app surfaces is already UTC
/// and only needs millisecond display precision, not full calendar-arithmetic correctness.
/// Uses Howard Hinnant's `civil_from_days` algorithm to turn days-since-epoch into y/m/d.
/// Shared by `now_rfc3339` (live stats/log timestamps) and `docker/images.rs` (converting
/// `ImageSummary.created`, which the Docker API reports as Unix seconds).
pub fn rfc3339_from_unix_secs(secs: i64, millis: u32) -> String {
    let secs = secs.max(0) as u64;
    let days = (secs / 86_400) as i64;
    let time_of_day = secs % 86_400;
    let (hour, minute, second) = (
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    );

    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };

    format!("{year:04}-{m:02}-{d:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}Z")
}

pub fn now_rfc3339() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    rfc3339_from_unix_secs(since_epoch.as_secs() as i64, since_epoch.subsec_millis())
}

fn map_state(state: Option<ContainerSummaryStateEnum>) -> ContainerStatus {
    match state {
        Some(ContainerSummaryStateEnum::CREATED) => ContainerStatus::Created,
        Some(ContainerSummaryStateEnum::RUNNING) => ContainerStatus::Running,
        Some(ContainerSummaryStateEnum::PAUSED) => ContainerStatus::Paused,
        Some(ContainerSummaryStateEnum::EXITED) => ContainerStatus::Exited,
        _ => ContainerStatus::Stopped,
    }
}

fn map_ports(ports: Option<Vec<PortSummary>>) -> Vec<ContainerPort> {
    ports
        .unwrap_or_default()
        .into_iter()
        .filter_map(|p| {
            let protocol = match p.typ {
                Some(PortSummaryTypeEnum::TCP) => PortProtocol::Tcp,
                Some(PortSummaryTypeEnum::UDP) => PortProtocol::Udp,
                _ => return None,
            };
            Some(ContainerPort {
                container_port: p.private_port,
                host_port: p.public_port,
                protocol,
            })
        })
        .collect()
}

/// Maps bollard's `ContainerSummary` onto our `Container` model. Uptime is approximated from
/// `created` rather than the container's exact start time (which would need a per-container
/// `inspect_container` call that a plain list doesn't need otherwise).
pub fn to_container(summary: ContainerSummary, profile_name: &str) -> Container {
    let status = map_state(summary.state);
    let uptime_seconds = if status == ContainerStatus::Running {
        summary.created.map(|created| {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs() as i64)
                .unwrap_or(created);
            (now - created).max(0)
        })
    } else {
        None
    };

    Container {
        id: summary.id.unwrap_or_default(),
        name: summary
            .names
            .and_then(|names| names.into_iter().next())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_default(),
        image: summary.image.unwrap_or_default(),
        status,
        uptime_seconds,
        ports: map_ports(summary.ports),
        latest_stats: None,
        profile_name: profile_name.to_string(),
    }
}

/// Computes CPU%/memory from one stats sample using the same delta formula `docker stats`
/// itself uses: `(cpu_delta / system_delta) * online_cpus * 100`. Returns `None` when the
/// sample is missing the fields the formula needs (e.g. a container's very first sample).
pub fn compute_latest_stats(
    stats: &ContainerStatsResponse,
    sampled_at: String,
) -> Option<ContainerLatestStats> {
    let cpu_stats = stats.cpu_stats.as_ref()?;
    let total_usage = cpu_stats.cpu_usage.as_ref()?.total_usage?;
    let system_usage = cpu_stats.system_cpu_usage?;
    let online_cpus = cpu_stats.online_cpus.unwrap_or(1).max(1) as f64;

    let (pre_total_usage, pre_system_usage) = stats
        .precpu_stats
        .as_ref()
        .map(|pre| {
            (
                pre.cpu_usage
                    .as_ref()
                    .and_then(|u| u.total_usage)
                    .unwrap_or(0),
                pre.system_cpu_usage.unwrap_or(0),
            )
        })
        .unwrap_or((0, 0));

    let cpu_delta = total_usage.saturating_sub(pre_total_usage) as f64;
    let system_delta = system_usage.saturating_sub(pre_system_usage) as f64;
    let cpu_percent = if system_delta > 0.0 {
        (cpu_delta / system_delta) * online_cpus * 100.0
    } else {
        0.0
    };

    let memory_stats = stats.memory_stats.as_ref()?;

    Some(ContainerLatestStats {
        cpu_percent,
        memory_bytes: memory_stats.usage.unwrap_or(0),
        memory_limit_bytes: memory_stats.limit.unwrap_or(0),
        sampled_at,
    })
}

// ---------------------------------------------------------------------------
// Connection (the only place a validated socket path becomes a live `Docker` client)
// ---------------------------------------------------------------------------

pub async fn connect(socket_path: &str, profile_name: &str) -> Result<Docker, IpcError> {
    let resolved = resolve_for_connection(socket_path, profile_name)?;
    let resolved_str = resolved.to_str().ok_or_else(|| {
        IpcError::new(
            "SOCKET_UNREACHABLE",
            "Docker socket path is not valid UTF-8",
        )
    })?;
    Docker::connect_with_unix(
        resolved_str,
        CONNECT_TIMEOUT_SECS,
        bollard::API_DEFAULT_VERSION,
    )
    .map_err(|err| {
        IpcError::with_detail(
            "SOCKET_UNREACHABLE",
            "Failed to connect to the Docker socket",
            err.to_string(),
        )
    })
}

/// Connects to `socket_path` (already validated for `profile_name`) and reports whether any
/// container is currently running on it. Any connection or API failure is treated as "no
/// running containers" — an unreachable socket cannot itself justify blocking a delete.
pub async fn has_running_containers(socket_path: &str, profile_name: &str) -> bool {
    let Ok(docker) = connect(socket_path, profile_name).await else {
        return false;
    };
    let options = ListContainersOptions {
        all: false,
        ..Default::default()
    };
    match docker.list_containers(Some(options)).await {
        Ok(containers) => !containers.is_empty(),
        Err(_) => false,
    }
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

fn api_error(action: &str, id: &str, err: bollard::errors::Error) -> IpcError {
    IpcError::with_detail(
        "DOCKER_API_ERROR",
        format!("Failed to {action} container '{id}'"),
        err.to_string(),
    )
}

pub async fn list(docker: &Docker, profile_name: &str) -> Result<Vec<Container>, IpcError> {
    let options = ListContainersOptions {
        all: true,
        ..Default::default()
    };
    let summaries = docker.list_containers(Some(options)).await.map_err(|err| {
        IpcError::with_detail(
            "DOCKER_API_ERROR",
            "Failed to list containers",
            err.to_string(),
        )
    })?;
    Ok(summaries
        .into_iter()
        .map(|s| to_container(s, profile_name))
        .collect())
}

/// "All profiles" fan-out: queries every already-connected client concurrently and
/// concatenates the `profileName`-tagged rows. A single unreachable profile's error is
/// swallowed rather than failing the whole call — the caller already validated every entry
/// here is a currently-`Running` profile, so a failure at this point is a transient
/// per-profile connectivity issue, not something the whole "all profiles" view should abort
/// over.
pub async fn list_many(targets: &[(String, Arc<Docker>)]) -> Vec<Container> {
    let futures = targets.iter().map(|(name, docker)| list(docker, name));
    let results = futures_util::future::join_all(futures).await;
    results
        .into_iter()
        .flat_map(|result| result.unwrap_or_default())
        .collect()
}

pub async fn start(docker: &Docker, id: &str) -> Result<(), IpcError> {
    docker
        .start_container(id, None)
        .await
        .map_err(|err| api_error("start", id, err))
}

pub async fn stop(docker: &Docker, id: &str) -> Result<(), IpcError> {
    docker
        .stop_container(id, None::<StopContainerOptions>)
        .await
        .map_err(|err| api_error("stop", id, err))
}

pub async fn remove(docker: &Docker, id: &str) -> Result<(), IpcError> {
    let options = RemoveContainerOptions {
        force: true,
        ..Default::default()
    };
    docker
        .remove_container(id, Some(options))
        .await
        .map_err(|err| api_error("remove", id, err))
}

// ---------------------------------------------------------------------------
// Background streaming tasks
// ---------------------------------------------------------------------------

/// Spawns the per-profile stats-streaming task: every second, lists the profile's running
/// containers and fetches one non-streaming, two-sample stats snapshot per container (the
/// same `stream: false, one_shot: false` Docker's own `docker stats --no-stream` uses, so
/// `cpu_stats`/`precpu_stats` are both populated for an accurate delta), emitting
/// `container-stats` for each. Runs until aborted (profile switch tears it down via
/// `state::teardown_profile_resources`).
pub fn spawn_stats_watcher(
    app: AppHandle,
    docker: Arc<Docker>,
    _profile_name: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(STATS_POLL_INTERVAL);
        loop {
            interval.tick().await;

            let running = ListContainersOptions {
                all: false,
                ..Default::default()
            };
            let Ok(summaries) = docker.list_containers(Some(running)).await else {
                continue;
            };

            let fetches = summaries.into_iter().filter_map(|s| s.id).map(|id| {
                let docker = docker.clone();
                async move {
                    let options = StatsOptions {
                        stream: false,
                        one_shot: false,
                    };
                    let mut stream = docker.stats(&id, Some(options));
                    stream.next().await.map(|result| (id, result))
                }
            });

            for outcome in futures_util::future::join_all(fetches).await {
                let Some((id, Ok(response))) = outcome else {
                    continue;
                };
                let sampled_at = now_rfc3339();
                let Some(latest) = compute_latest_stats(&response, sampled_at.clone()) else {
                    continue;
                };
                let sample = ResourceUsageSample {
                    container_id: id.clone(),
                    cpu_percent: latest.cpu_percent,
                    memory_bytes: latest.memory_bytes,
                    sampled_at,
                };
                let _ = app.emit(
                    "container-stats",
                    serde_json::json!({ "containerId": id, "sample": sample }),
                );
            }
        }
    })
}

/// Spawns the log-follow task backing `subscribe_container_logs`. Docker prepends an
/// RFC-3339 timestamp to each line when `timestamps: true` (split off below) so no
/// date/time dependency is needed here either.
pub fn spawn_log_watcher(
    app: AppHandle,
    docker: Arc<Docker>,
    container_id: String,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        let options = LogsOptions {
            follow: true,
            stdout: true,
            stderr: true,
            timestamps: true,
            tail: "100".to_string(),
            ..Default::default()
        };
        let mut stream = docker.logs(&container_id, Some(options));

        while let Some(chunk) = stream.next().await {
            let Ok(output) = chunk else { break };
            let (stream_name, bytes) = match &output {
                bollard::container::LogOutput::StdOut { message } => ("stdout", message),
                bollard::container::LogOutput::StdErr { message } => ("stderr", message),
                bollard::container::LogOutput::StdIn { message } => ("stdout", message),
                bollard::container::LogOutput::Console { message } => ("stdout", message),
            };

            let raw = String::from_utf8_lossy(bytes);
            let trimmed = raw.trim_end_matches('\n');
            let (timestamp, line) = match trimmed.split_once(' ') {
                Some((ts, rest)) => (ts.to_string(), rest.to_string()),
                None => (now_rfc3339(), trimmed.to_string()),
            };

            let _ = app.emit(
                "container-log-line",
                serde_json::json!({
                    "containerId": container_id,
                    "stream": stream_name,
                    "line": line,
                    "timestamp": timestamp,
                }),
            );
        }
    })
}

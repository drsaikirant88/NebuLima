//! Docker events watcher. Subscribes to the active profile's Docker event stream filtered
//! to container/image/volume events and re-emits
//! `containers-changed`/`images-changed`/`volumes-changed` with a fresh list whenever
//! something is created/started/stopped/removed etc. outside of this app's own commands
//! (e.g. via `docker` CLI, `colima`, or another tool).
use crate::docker::{containers, images, volumes};
use bollard::models::EventMessageTypeEnum;
use bollard::query_parameters::EventsOptions;
use bollard::Docker;
use futures_util::StreamExt;
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio::task::JoinHandle;

/// Payload shape for `containers-changed`/`images-changed`/`volumes-changed` — scoped to
/// the one profile whose underlying engine changed, so an "All profiles" view can merge
/// just this slice in rather than replacing its whole combined list, and a view filtered to
/// a different profile can ignore it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceChangedPayload<T: Serialize + Clone> {
    pub profile_name: String,
    pub items: Vec<T>,
}

pub fn spawn_watcher(app: AppHandle, docker: Arc<Docker>, profile_name: String) -> JoinHandle<()> {
    tokio::spawn(async move {
        let mut filters = HashMap::new();
        filters.insert(
            "type".to_string(),
            vec![
                "container".to_string(),
                "image".to_string(),
                "volume".to_string(),
            ],
        );
        let options = EventsOptions {
            filters: Some(filters),
            ..Default::default()
        };

        let mut stream = docker.events(Some(options));
        while let Some(event) = stream.next().await {
            let Ok(message) = event else { continue };
            match message.typ {
                Some(EventMessageTypeEnum::CONTAINER) => {
                    if let Ok(updated) = containers::list(&docker, &profile_name).await {
                        let payload = ResourceChangedPayload {
                            profile_name: profile_name.clone(),
                            items: updated,
                        };
                        let _ = app.emit("containers-changed", payload);
                    }
                }
                Some(EventMessageTypeEnum::IMAGE) => {
                    if let Ok(updated) = images::list(&docker, &profile_name).await {
                        let payload = ResourceChangedPayload {
                            profile_name: profile_name.clone(),
                            items: updated,
                        };
                        let _ = app.emit("images-changed", payload);
                    }
                }
                Some(EventMessageTypeEnum::VOLUME) => {
                    if let Ok(updated) = volumes::list(&docker, &profile_name).await {
                        let payload = ResourceChangedPayload {
                            profile_name: profile_name.clone(),
                            items: updated,
                        };
                        let _ = app.emit("volumes-changed", payload);
                    }
                }
                _ => {}
            }
        }
    })
}

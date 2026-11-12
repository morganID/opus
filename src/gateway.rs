//! The gateway half: one instance, started once, asked about itself cheaply.
//!
//! `xproxy::Gateway` owns the engine's lifetime; this owns the *runtime* it
//! needs, because the menu is drawn on the main thread and the engine is
//! async. Everything that can block goes to the runtime and the menu reads a
//! mutex-protected copy of the last answer — a status poll that took 200 ms
//! would be a menu that stutters every time it opens.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use xproxy::Status;

/// What the menu shows and what it can do.
pub struct GatewayHandle {
    runtime: tokio::runtime::Runtime,
    gateway: Arc<xproxy::Gateway>,
    status: Arc<Mutex<Option<Status>>>,
    models: Arc<Mutex<usize>>,
}

impl GatewayHandle {
    /// Start the engine in the background and begin polling its status.
    pub fn spawn() -> Result<Self, Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()?;
        let gateway = Arc::new(xproxy::Gateway::local()?);
        let status = Arc::new(Mutex::new(None));
        let models = Arc::new(Mutex::new(0));

        // The engine takes seconds on a cold start (keychain, schema, socket).
        // The icon is already on screen by then, showing "…".
        {
            let gateway = gateway.clone();
            let status = status.clone();
            runtime.spawn(async move {
                match gateway.start().await {
                    Ok(state) => *status.lock().unwrap() = Some(state),
                    Err(error) => {
                        tracing::error!(%error, "the gateway did not start");
                        let mut state = gateway.status().await;
                        state.error = Some(error.message);
                        *status.lock().unwrap() = Some(state);
                    }
                }
            });
        }

        // The poller. Three seconds is a compromise: fast enough that pausing
        // from the menu looks immediate, slow enough that an idle menu bar is
        // not a database client.
        {
            let gateway = gateway.clone();
            let status = status.clone();
            let models = models.clone();
            runtime.spawn(async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                    let state = gateway.status().await;
                    let dir = PathBuf::from(&state.data_dir);
                    let catalog = xproxy::catalog::load(&dir);
                    *models.lock().unwrap() = catalog
                        .providers
                        .values()
                        .map(|entry| entry.models.len())
                        .sum();
                    *status.lock().unwrap() = Some(state);
                }
            });
        }

        Ok(Self {
            runtime,
            gateway,
            status,
            models,
        })
    }

    pub fn status(&self) -> Option<Status> {
        self.status.lock().ok().and_then(|status| status.clone())
    }

    pub fn models(&self) -> usize {
        *self.models.lock().unwrap()
    }

    /// Pause or resume, whichever the last status says is the useful one.
    pub fn toggle(&self) {
        let running = self.status().is_some_and(|status| status.running);
        let gateway = self.gateway.clone();
        self.runtime.spawn(async move {
            let result = if running {
                gateway.stop().await
            } else {
                gateway.start().await
            };
            match result {
                Ok(state) => tracing::info!(running = state.running, "the gateway changed state"),
                Err(error) => tracing::warn!(%error, "the gateway did not change state"),
            }
        });
    }

    /// Put the base URL or the gateway key on the pasteboard.
    pub fn copy(&self, what: Copy) {
        let gateway = self.gateway.clone();
        self.runtime.spawn(async move {
            let Ok(desktop) = gateway.desktop().await else {
                return;
            };
            let plane = desktop.data_plane();
            let text = match what {
                Copy::BaseUrl => plane.base_url.clone(),
                Copy::Key => plane.gateway_key.clone(),
            };
            if let Err(error) = crate::util::copy(&text) {
                tracing::warn!(%error, "the clipboard refused the write");
            }
        });
    }

    /// Run the token check the engine does not schedule for itself.
    pub fn refresh_tokens(&self) {
        let gateway = self.gateway.clone();
        self.runtime.spawn(async move {
            let Ok(desktop) = gateway.desktop().await else {
                return;
            };
            match xproxy::refresh::accounts(desktop.app(), false).await {
                Ok(outcomes) => {
                    let failed = outcomes
                        .iter()
                        .filter(|outcome| outcome.failure.is_some())
                        .count();
                    tracing::info!(checked = outcomes.len(), failed, "token check finished");
                }
                Err(error) => tracing::warn!(%error, "the token check did not run"),
            }
        });
    }
}

#[derive(Clone, Copy)]
pub enum Copy {
    BaseUrl,
    Key,
}

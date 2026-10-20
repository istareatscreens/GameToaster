use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::plugin::catalog::watcher::CatalogWatcher;

#[derive(Default)]
struct WatchState {
    watcher: Option<CatalogWatcher>,
    last_error: Option<String>,
}

pub(super) struct WatchControl {
    root: PathBuf,
    state: Mutex<WatchState>,
}

impl WatchControl {
    pub(super) fn start(root: PathBuf) -> Result<Arc<Self>, String> {
        let watcher = CatalogWatcher::start(&root)?;
        Ok(Arc::new(Self {
            root,
            state: Mutex::new(WatchState {
                watcher: Some(watcher),
                last_error: None,
            }),
        }))
    }

    pub(super) fn pause(&self) -> Result<(), String> {
        let watcher = self
            .state
            .lock()
            .map_err(|_| "catalog watcher lock is poisoned".to_string())?
            .watcher
            .take();
        drop(watcher);
        Ok(())
    }

    pub(super) fn resume(&self) {
        let mut state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        if state.watcher.is_some() {
            return;
        }
        match CatalogWatcher::start(&self.root) {
            Ok(watcher) => {
                state.watcher = Some(watcher);
                state.last_error = None;
            }
            Err(error) => state.last_error = Some(error),
        }
    }

    pub(super) fn take_dirty(&self) -> bool {
        self.state
            .lock()
            .ok()
            .and_then(|state| state.watcher.as_ref().map(CatalogWatcher::take_dirty))
            .unwrap_or(true)
    }

    #[cfg(test)]
    pub(super) fn is_running(&self) -> bool {
        self.state.lock().is_ok_and(|state| state.watcher.is_some())
    }
}

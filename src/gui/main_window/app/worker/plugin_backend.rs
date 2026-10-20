use crate::plugin::catalog::Catalog;
use crate::plugin::deployment::DeploymentEngine;
use crate::plugin::lifecycle::LifecycleService;

pub(super) struct PluginBackend {
    pub(super) catalog: Catalog,
    pub(super) deployment: DeploymentEngine,
    pub(super) lifecycle: LifecycleService,
}

pub(super) fn with_plugin_work<T>(
    backend: &Result<PluginBackend, String>,
    operation: impl FnOnce(&PluginBackend) -> Result<T, String>,
) -> Result<T, String> {
    let backend = backend.as_ref().map_err(Clone::clone)?;
    let _work = backend.lifecycle.activity().try_plugin_operation()?;
    operation(backend)
}

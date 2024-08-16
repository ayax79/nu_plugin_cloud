mod aws;
mod cache;
mod open;
mod stub;

use std::sync::{Mutex, MutexGuard};

use bytes::Bytes;
use cache::Cache;
use nu_plugin::Plugin;
use nu_protocol::{ShellError, Span, Spanned};
use tokio::runtime::Runtime;
use url::Url;

pub struct CloudPlugin {
    cache: Mutex<cache::Cache>,
    rt: Runtime,
}

impl Default for CloudPlugin {
    fn default() -> Self {
        CloudPlugin {
            cache: Mutex::new(cache::Cache::default()),
            rt: Runtime::new().expect("Could not create tokio runtime"),
        }
    }
}

impl CloudPlugin {
    pub fn cache_get(&self, url: &Spanned<Url>, span: Span) -> Result<Bytes, ShellError> {
        let mut lock = self.cache_lock()?;
        let bytes = self.rt.block_on(lock.get(url, span))?;
        drop(lock);
        Ok(bytes)
    }

    fn cache_lock(&self) -> Result<MutexGuard<Cache>, ShellError> {
        self.cache.lock().map_err(|e| ShellError::GenericError {
            error: format!("error acquiring cache lock: {e}"),
            msg: "".into(),
            span: None,
            help: None,
            inner: vec![],
        })
    }
}

impl Plugin for CloudPlugin {
    fn version(&self) -> String {
        env!("CARGO_PKG_VERSION").into()
    }

    fn commands(&self) -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = Self>>> {
        vec![Box::new(stub::Stub), Box::new(open::Open)]
    }
}

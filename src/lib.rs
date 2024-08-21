mod cache;
mod command;
mod providers;

use std::sync::{atomic::AtomicBool, Arc, Mutex, MutexGuard};

use bytes::Bytes;
use cache::Cache;
use nu_plugin::{EngineInterface, Plugin};
use nu_protocol::{HandlerGuard, ShellError, Signals, Span, Spanned};
use tokio::runtime::Runtime;
use url::Url;

pub struct CloudPlugin {
    cache: Mutex<cache::Cache>,
    pub(crate) rt: Runtime,
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
        command::commands()
    }
}

#[derive(Clone)]
struct SignalHandler {
    received: Arc<AtomicBool>,
    handler_guard: Option<HandlerGuard>,
}

impl SignalHandler {
    pub fn new(engine: &EngineInterface) -> Result<Self, ShellError> {
        let mut signal_handler = SignalHandler {
            received: Arc::new(AtomicBool::default()),
            handler_guard: None,
        };
        let cloned = signal_handler.clone();
        let guard = engine.register_signal_handler(Box::new(move |_| {
            cloned.signal_received();
        }))?;

        signal_handler.handler_guard = Some(guard);
        Ok(signal_handler)
    }

    pub fn signal_received(&self) {
        self.received
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

impl From<SignalHandler> for Signals {
    fn from(value: SignalHandler) -> Self {
        Signals::new(Arc::clone(&value.received))
    }
}

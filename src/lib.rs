mod cache;
mod command;
mod providers;

use cache::Cache;
use nu_plugin::{EngineInterface, Plugin};
use nu_protocol::{ShellError, Span, Spanned};
use object_store::path::Path;
use providers::NuObjectStore;
use tokio::runtime::Runtime;
use url::Url;

pub struct CloudPlugin {
    pub cache: cache::Cache,
    pub rt: Runtime,
}

impl Default for CloudPlugin {
    fn default() -> Self {
        CloudPlugin {
            cache: Cache::default(),
            rt: Runtime::new().expect("Could not create tokio runtime"),
        }
    }
}

impl CloudPlugin {
    pub async fn parse_url(
        &self,
        engine: &EngineInterface,
        url: &Spanned<Url>,
        span: Span,
    ) -> Result<(NuObjectStore, Path), ShellError> {
        providers::parse_url(engine, &self.cache, url, span).await
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

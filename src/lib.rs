mod aws;
mod cache;
mod open;
mod stub;

use std::sync::{Mutex, MutexGuard};

use bytes::Bytes;
use cache::Cache;
use nu_plugin::Plugin;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::{parse_url_opts, path::Path, ObjectStore, ObjectStoreScheme};
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

pub async fn parse_url(
    url: &Spanned<Url>,
    span: Span,
) -> Result<(Box<dyn ObjectStore>, Path), ShellError> {
    let (scheme, _) =
        ObjectStoreScheme::parse(&url.item).map_err(|e| ShellError::IncorrectValue {
            msg: format!("Unsupported url: {e}"),
            val_span: url.span,
            call_span: span,
        })?;

    let options: Vec<(String, String)> = match scheme {
        ObjectStoreScheme::AmazonS3 => aws::options().await?,
        _ => {
            vec![]
        }
    };

    parse_url_opts(&url.item, options).map_err(|e| ShellError::IncorrectValue {
        msg: format!("Unsupported url: {e}"),
        val_span: url.span,
        call_span: span,
    })
}

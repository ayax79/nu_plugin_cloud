mod aws;
mod local;
mod mem;

use crate::cache::Cache;
use nu_plugin::EngineInterface;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::{path::Path, ObjectStore, ObjectStoreScheme};
use std::sync::Arc;
use url::Url;

#[derive(Clone)]
pub enum NuObjectStore {
    Local(Arc<dyn ObjectStore>),
    Memory(Arc<dyn ObjectStore>),
    AmazonS3 {
        store: Arc<dyn ObjectStore>,
        bucket: String,
        region: String,
    },
    #[allow(dead_code)]
    GoogleCloudStorage(Arc<dyn ObjectStore>),
    #[allow(dead_code)]
    MicrosoftAzure(Arc<dyn ObjectStore>),
    #[allow(dead_code)]
    Http(Arc<dyn ObjectStore>),
}
impl NuObjectStore {
    pub fn object_store(&self) -> &dyn ObjectStore {
        match self {
            NuObjectStore::Local(store) => store.as_ref(),
            NuObjectStore::Memory(store) => store.as_ref(),
            NuObjectStore::AmazonS3 { store, .. } => store.as_ref(),
            NuObjectStore::GoogleCloudStorage(store) => store.as_ref(),
            NuObjectStore::MicrosoftAzure(store) => store.as_ref(),
            NuObjectStore::Http(store) => store.as_ref(),
        }
    }
}

pub async fn parse_url(
    engine: &EngineInterface,
    cache: &Cache,
    url: &Spanned<Url>,
    span: Span,
) -> Result<(NuObjectStore, Path), ShellError> {
    let (scheme, path) =
        ObjectStoreScheme::parse(&url.item).map_err(|e| ShellError::IncorrectValue {
            msg: format!("Unsupported url: {e}"),
            val_span: url.span,
            call_span: span,
        })?;

    let path = Path::parse(path).map_err(|e| ShellError::IncorrectValue {
        msg: format!("Unsupported path: {e}"),
        val_span: url.span,
        call_span: span,
    })?;

    let object_store = match scheme {
        ObjectStoreScheme::AmazonS3 => aws::build_object_store(engine, cache, url).await?,
        ObjectStoreScheme::Local => local::build_object_store(engine, cache).await?,
        ObjectStoreScheme::Memory => mem::build_object_store(engine, cache).await?,
        _ => {
            return Err(ShellError::IncorrectValue {
                msg: format!("Unsupported url: {}", url.item),
                val_span: url.span,
                call_span: span,
            })
        }
    };

    Ok((object_store, path))
}

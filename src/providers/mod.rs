mod aws;
mod gcs;
mod local;
mod mem;

use crate::cache::Cache;
use nu_plugin::EngineInterface;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::{ObjectStore, ObjectStoreScheme, path::Path};
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
    GoogleCloudStorage {
        store: Arc<dyn ObjectStore>,
        bucket: String,
    },
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
            NuObjectStore::GoogleCloudStorage { store, .. } => store.as_ref(),
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
        ObjectStoreScheme::GoogleCloudStorage => {
            gcs::build_object_store(engine, cache, url).await?
        }
        ObjectStoreScheme::Local => local::build_object_store(engine, cache).await?,
        ObjectStoreScheme::Memory => mem::build_object_store(engine, cache).await?,
        _ => {
            return Err(ShellError::IncorrectValue {
                msg: format!("Unsupported url: {}", url.item),
                val_span: url.span,
                call_span: span,
            });
        }
    };

    Ok((object_store, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gs_url_maps_to_google_cloud_storage() {
        let url = Url::parse("gs://test-bucket/path/to/file.txt").unwrap();
        let (scheme, path) = ObjectStoreScheme::parse(&url).unwrap();

        assert!(matches!(scheme, ObjectStoreScheme::GoogleCloudStorage));
        assert_eq!(path.as_ref(), "path/to/file.txt");
    }

    #[test]
    fn test_gs_url_without_host_is_not_supported() {
        let url = Url::parse("gs://").unwrap();
        assert!(ObjectStoreScheme::parse(&url).is_err());
    }

    #[test]
    fn test_s3_url_does_not_map_to_google_cloud_storage() {
        let url = Url::parse("s3://test-bucket/path/to/file.txt").unwrap();
        let (scheme, _) = ObjectStoreScheme::parse(&url).unwrap();

        assert!(!matches!(scheme, ObjectStoreScheme::GoogleCloudStorage));
    }
}

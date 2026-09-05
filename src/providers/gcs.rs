use std::sync::Arc;

use nu_plugin::EngineInterface;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::ObjectStore;
use object_store::gcp::GoogleCloudStorageBuilder;
use url::Url;

use crate::cache::{Cache, ObjectStoreCacheKey};

use super::NuObjectStore;

pub async fn build_object_store(
    engine: &EngineInterface,
    cache: &Cache,
    url: &Spanned<Url>,
) -> Result<NuObjectStore, ShellError> {
    let bucket = parse_bucket(&url.item).ok_or_else(|| ShellError::GenericError {
        error: format!(
            "Could not determine Google Cloud Storage bucket name from url {}",
            url.item
        ),
        msg: "".into(),
        span: Some(url.span),
        help: None,
        inner: vec![],
    })?;

    let cache_key = ObjectStoreCacheKey::GoogleCloudStorage {
        bucket: bucket.clone(),
    };

    if let Some(object_store) = cache.get_store(&cache_key).await {
        Ok(object_store)
    } else {
        let store = build_store(GoogleCloudStorageBuilder::from_env(), &url.item, url.span)?;

        let object_store = NuObjectStore::GoogleCloudStorage { store, bucket };

        cache
            .put_store(engine, cache_key, object_store.clone())
            .await?;
        Ok(object_store)
    }
}

fn build_store(
    builder: GoogleCloudStorageBuilder,
    url: &Url,
    span: Span,
) -> Result<Arc<dyn ObjectStore>, ShellError> {
    let gcs = builder
        .with_url(url.to_string())
        .build()
        .map_err(|e| ShellError::GenericError {
            error: format!("Could not create Google Cloud Storage client: {e}"),
            msg: "".into(),
            span: Some(span),
            help: None,
            inner: vec![],
        })?;

    Ok(Arc::new(gcs))
}

fn parse_bucket(url: &Url) -> Option<String> {
    if url.scheme() != "gs" {
        return None;
    }

    url.host_str()
        .filter(|bucket| !bucket.is_empty())
        .map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FAKE_KEY: &str = r#"{"private_key": "private_key", "private_key_id": "private_key_id", "client_email":"client_email", "disable_oauth":true}"#;

    #[test]
    fn test_parse_bucket() {
        assert_eq!(
            parse_bucket(&Url::parse("gs://my-bucket/path/to/file.txt").unwrap()),
            Some("my-bucket".to_string())
        );
        assert_eq!(
            parse_bucket(&Url::parse("gs://my-bucket").unwrap()),
            Some("my-bucket".to_string())
        );
        assert_eq!(
            parse_bucket(&Url::parse("gs:///path/to/file.txt").unwrap()),
            None
        );
        assert_eq!(
            parse_bucket(&Url::parse("s3://my-bucket/path/to/file.txt").unwrap()),
            None
        );
    }

    #[test]
    fn test_build_store_without_network_access() {
        let url = Url::parse("gs://test-bucket/path/to/file.txt").unwrap();
        let builder = GoogleCloudStorageBuilder::new().with_service_account_key(FAKE_KEY);
        let store = build_store(builder, &url, Span::test_data()).unwrap();

        let object_store = NuObjectStore::GoogleCloudStorage {
            store,
            bucket: "test-bucket".to_string(),
        };
        assert_eq!(
            ObjectStoreCacheKey::from(&object_store),
            ObjectStoreCacheKey::GoogleCloudStorage {
                bucket: "test-bucket".to_string()
            }
        );
    }

    #[test]
    fn test_build_store_invalid_service_account_key() {
        let url = Url::parse("gs://test-bucket/path/to/file.txt").unwrap();
        let builder = GoogleCloudStorageBuilder::new().with_service_account_key("not json");
        let error = build_store(builder, &url, Span::test_data()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Could not create Google Cloud Storage client")
        );
    }

    #[test]
    fn test_build_store_missing_bucket_name() {
        let url = Url::parse("gs://").unwrap();
        let builder = GoogleCloudStorageBuilder::new().with_service_account_key(FAKE_KEY);
        let error = build_store(builder, &url, Span::test_data()).unwrap_err();

        assert!(
            error
                .to_string()
                .contains("Could not create Google Cloud Storage client")
        );
    }
}

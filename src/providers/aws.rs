use std::{error::Error, sync::Arc};

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use nu_protocol::{ShellError, Spanned};
use object_store::aws::{AmazonS3Builder, AmazonS3ConfigKey};
use url::Url;

use crate::cache::{Cache, ObjectStoreCacheKey};

use super::NuObjectStore;

pub async fn parse_url(cache: &Cache, url: &Spanned<Url>) -> Result<NuObjectStore, ShellError> {
    let aws_config = aws_load_config().await;
    let builder = AmazonS3Builder::new().with_url(url.item.clone());

    let bucket = builder
        .get_config_value(&AmazonS3ConfigKey::Bucket)
        .ok_or_else(|| ShellError::GenericError {
            error: format!(
                "Could not determine Amazon S3 bucket name from url {}",
                url.item
            ),
            msg: "".into(),
            span: Some(url.span),
            help: None,
            inner: vec![],
        })?;

    let (builder, region) = if let Some(region) = aws_config.region() {
        (builder.with_region(region.to_string()), region.to_string())
    } else {
        return Err(ShellError::GenericError {
            error: "Could not determine AWS region from environment".into(),
            msg: "".into(),
            span: Some(url.span),
            help: None,
            inner: vec![],
        });
    };

    let cache_key = ObjectStoreCacheKey::AmazonS3 {
        bucket: bucket.clone(),
        region: region.clone(),
    };

    if let Some(object_store) = cache.get_store(&cache_key)? {
        Ok(object_store)
    } else {
        let builder = if let Some(credentials) = aws_creds(&aws_config).await? {
            let builder = builder
                .with_access_key_id(credentials.access_key_id())
                .with_secret_access_key(credentials.secret_access_key());

            if let Some(token) = credentials.session_token() {
                builder.with_token(token)
            } else {
                builder
            }
        } else {
            return Err(ShellError::GenericError {
                error: "Could not determine AWS credentials from environment".into(),
                msg: "".into(),
                span: Some(url.span),
                help: None,
                inner: vec![],
            });
        };

        let s3 = builder.build().map_err(|e| ShellError::GenericError {
            error: format!("Could not create Amazon S3 client: {e}"),
            msg: "".into(),
            span: Some(url.span),
            help: None,
            inner: vec![],
        })?;

        let object_store = NuObjectStore::AmazonS3 {
            store: Arc::new(s3),
            bucket,
            region,
        };

        cache.put_store(cache_key, object_store.clone())?;
        Ok(object_store)
    }
}

async fn aws_load_config() -> SdkConfig {
    aws_config::load_defaults(BehaviorVersion::latest()).await
}

async fn aws_creds(aws_config: &SdkConfig) -> Result<Option<Credentials>, ShellError> {
    if let Some(provider) = aws_config.credentials_provider() {
        Ok(Some(provider.provide_credentials().await.map_err(|e| {
            ShellError::GenericError {
                error: format!(
                    "Could not fetch AWS credentials: {} - {}",
                    e,
                    e.source()
                        .map(|e| format!("{}", e))
                        .unwrap_or("".to_string())
                ),
                msg: "".into(),
                span: None,
                help: None,
                inner: vec![],
            }
        })?))
    } else {
        Ok(None)
    }
}

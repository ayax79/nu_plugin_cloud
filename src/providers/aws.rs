use std::{error::Error, sync::Arc};

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use itertools::Itertools;
use nu_protocol::{ShellError, Spanned};
use object_store::aws::AmazonS3Builder;
use url::Url;

use crate::cache::{Cache, ObjectStoreCacheKey};

use super::NuObjectStore;

pub async fn build_object_store(
    cache: &Cache,
    url: &Spanned<Url>,
) -> Result<NuObjectStore, ShellError> {
    let aws_config = aws_load_config().await;

    let parsed_info = parse_url_parts(&url.item);

    let bucket = parsed_info
        .bucket
        .clone()
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

    let region = if let Some(region) = aws_config
        .region()
        .map(ToString::to_string)
        .or(parsed_info.region)
    {
        region
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

    if let Some(object_store) = cache.get_store(&cache_key).await {
        Ok(object_store)
    } else {
        let builder = AmazonS3Builder::new()
            .with_url(url.item.clone())
            .with_region(region.clone());

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

        cache.put_store(cache_key, object_store.clone()).await;
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

#[derive(Default)]
struct ParsedInfo {
    bucket: Option<String>,
    region: Option<String>,
}

// This is borrowed from the iternals of the AmazonS3 builder.
// Unforunately, neither builder or the object store expose the bucket and region
// name and they are needed for caching
fn parse_url_parts(url: &Url) -> ParsedInfo {
    let host = url.host_str().unwrap_or_default();

    match url.scheme() {
        "s3" | "s3a" => ParsedInfo {
            bucket: Some(host.to_string()),
            region: None,
        },
        "https" => match host.splitn(4, '.').collect_tuple() {
            Some(("s3", region, "amazonaws", "com")) => {
                let region = Some(region.to_string());
                let bucket = url.path_segments().into_iter().flatten().next();

                ParsedInfo {
                    bucket: bucket.map(|s| s.to_string()),
                    region,
                }
            }
            Some((bucket, "s3", region, "amazonaws.com")) => {
                let bucket = Some(bucket.to_string());
                let region = Some(region.to_string());

                ParsedInfo { bucket, region }
            }
            Some((_account, "r2", "cloudflarestorage", "com")) => {
                let region = Some("auto".to_string());
                let bucket = url
                    .path_segments()
                    .into_iter()
                    .flatten()
                    .next()
                    .map(ToString::to_string);
                ParsedInfo { bucket, region }
            }
            _ => ParsedInfo::default(),
        },
        _scheme => ParsedInfo::default(),
    }
}

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use aws_config::{BehaviorVersion, SdkConfig};
use aws_credential_types::{provider::ProvideCredentials, Credentials};
use bytes::Bytes;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::{
    aws::AmazonS3ConfigKey, parse_url_opts, path::Path, Error as ObjectStoreError, GetOptions,
    ObjectStore, ObjectStoreScheme,
};
use url::Url;

pub struct CacheEntry {
    path: Path,
    /// Data returned by last request
    data: Bytes,
    /// ETag identifying the object returned by the server
    e_tag: String,
    /// Instant of last refresh
    refreshed_at: Instant,
    /// Object store used for this file.
    /// todo: ideally there would be a way to reuse this for multiple paths in a generic way
    store: Box<dyn ObjectStore>,
}

/// Example cache that checks entries after 10 seconds for a new version
#[derive(Default)]
pub struct Cache {
    entries: HashMap<Url, CacheEntry>,
}

impl Cache {
    pub async fn get(&mut self, url: &Spanned<Url>, span: Span) -> Result<Bytes, ShellError> {
        Ok(match self.entries.get_mut(&url.item) {
            Some(e) => match e.refreshed_at.elapsed() < Duration::from_secs(10) {
                true => e.data.clone(), // Return cached data
                false => {
                    // Check if remote version has changed
                    let opts = GetOptions {
                        if_none_match: Some(e.e_tag.clone()),
                        ..GetOptions::default()
                    };
                    match e.store.get_opts(&e.path, opts).await {
                        Ok(d) => e.data = d.bytes().await.map_err(cache_get_error)?,
                        Err(ObjectStoreError::NotModified { .. }) => {} // Data has not changed
                        Err(e) => return Err(cache_get_error(e)),
                    }
                    e.refreshed_at = Instant::now();
                    e.data.clone()
                }
            },
            None => {
                // Not cached, fetch data
                let (store, path) = parse_url(url, span).await?;
                let get = store.get(&path).await.map_err(cache_get_error)?;
                let e_tag = get.meta.e_tag.clone();
                let data = get.bytes().await.map_err(cache_get_error)?;
                if let Some(e_tag) = e_tag {
                    let entry = CacheEntry {
                        path,
                        e_tag,
                        data: data.clone(),
                        refreshed_at: Instant::now(),
                        store,
                    };
                    self.entries.insert(url.item.clone(), entry);
                }
                data
            }
        })
    }
}

fn cache_get_error(e: impl std::error::Error) -> ShellError {
    ShellError::GenericError {
        error: format!("Error fetching data from obect store: {}", e),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
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
        ObjectStoreScheme::AmazonS3 => build_aws_options().await?,
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

async fn build_aws_options() -> Result<Vec<(String, String)>, ShellError> {
    let aws_config = aws_load_config().await;
    let mut options: Vec<(String, String)> = Vec::with_capacity(3);
    if let Some(region) = aws_config.region() {
        options.push((
            AmazonS3ConfigKey::Region.as_ref().to_string(),
            region.to_string(),
        ));
    }
    if let Some(credentials) = aws_creds(&aws_config).await? {
        options.push((
            AmazonS3ConfigKey::AccessKeyId.as_ref().to_string(),
            credentials.access_key_id().to_string(),
        ));
        options.push((
            AmazonS3ConfigKey::SecretAccessKey.as_ref().to_string(),
            credentials.secret_access_key().to_string(),
        ));
    }

    Ok(options)
}

async fn aws_load_config() -> SdkConfig {
    aws_config::load_defaults(BehaviorVersion::latest()).await
}

async fn aws_creds(aws_config: &SdkConfig) -> Result<Option<Credentials>, ShellError> {
    if let Some(provider) = aws_config.credentials_provider() {
        Ok(Some(provider.provide_credentials().await.map_err(|e| {
            ShellError::GenericError {
                error: format!("Could not fetch AWS credentials: {e}"),
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

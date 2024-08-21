use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use bytes::Bytes;
use nu_protocol::{ShellError, Span, Spanned};
use object_store::{path::Path, Error as ObjectStoreError, GetOptions, ObjectStore};
use url::Url;

use crate::providers::parse_url;

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

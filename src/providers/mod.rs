mod aws;

use nu_protocol::{ShellError, Span, Spanned};
use object_store::parse_url_opts;
use object_store::{path::Path, ObjectStore, ObjectStoreScheme};
use url::Url;

pub enum NuObjectStore {
    Local(Box<dyn ObjectStore>),
    Memory(Box<dyn ObjectStore>),
    AmazonS3(Box<dyn ObjectStore>),
    #[allow(dead_code)]
    GoogleCloudStorage(Box<dyn ObjectStore>),
    #[allow(dead_code)]
    MicrosoftAzure(Box<dyn ObjectStore>),
    #[allow(dead_code)]
    Http(Box<dyn ObjectStore>),
}
impl NuObjectStore {
    pub fn new(scheme: ObjectStoreScheme, object_store: Box<dyn ObjectStore>) -> Self {
        match scheme {
            ObjectStoreScheme::Local => NuObjectStore::Local(object_store),
            ObjectStoreScheme::Memory => NuObjectStore::Memory(object_store),
            ObjectStoreScheme::AmazonS3 => NuObjectStore::AmazonS3(object_store),
            _ => unimplemented!(),
        }
    }

    pub fn object_store(&self) -> &dyn ObjectStore {
        match self {
            NuObjectStore::Local(store) => store.as_ref(),
            NuObjectStore::Memory(store) => store.as_ref(),
            NuObjectStore::AmazonS3(store) => store.as_ref(),
            NuObjectStore::GoogleCloudStorage(store) => store.as_ref(),
            NuObjectStore::MicrosoftAzure(store) => store.as_ref(),
            NuObjectStore::Http(store) => store.as_ref(),
        }
    }
}

pub async fn parse_url(
    url: &Spanned<Url>,
    span: Span,
) -> Result<(NuObjectStore, Path), ShellError> {
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

    let (object_store, path) =
        parse_url_opts(&url.item, options).map_err(|e| ShellError::IncorrectValue {
            msg: format!("Unsupported url: {e}"),
            val_span: url.span,
            call_span: span,
        })?;

    Ok((NuObjectStore::new(scheme, object_store), path))
}

mod aws;

use nu_protocol::{ShellError, Span, Spanned};
use object_store::parse_url_opts;
use object_store::{path::Path, ObjectStore, ObjectStoreScheme};
use url::Url;

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

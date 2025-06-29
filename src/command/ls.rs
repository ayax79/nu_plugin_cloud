use std::{path::PathBuf, str::FromStr, vec};

use futures::StreamExt;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type, Value, record,
};
use url::Url;

use crate::CloudPlugin;

pub struct Ls;

impl PluginCommand for Ls {
    type Plugin = CloudPlugin;

    fn name(&self) -> &str {
        "cloud ls"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cloud ls")
            .required("uri", SyntaxShape::String, "The url to use.")
            .category(Category::FileSystem)
            .input_output_types(vec![(Type::Nothing, Type::Any)])
    }

    fn description(&self) -> &str {
        "List the filenames, sizes, modificationtime , etags, and versions of a cloud location."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "List the files in a s3 bucket.",
            example: "cloud ls s3://mybucket",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(command(engine, plugin, call))
            .map_err(LabeledError::from)
    }
}

async fn command(
    engine: &EngineInterface,
    plugin: &CloudPlugin,
    call: &EvaluatedCall,
) -> Result<PipelineData, ShellError> {
    let call_span = call.head;
    let spanned_path: Spanned<PathBuf> = call.req(0)?;
    let url_path = spanned_path.item;
    let url = url_path
        .to_str()
        .expect("The path should already be unicode")
        .to_string();
    let url = Spanned {
        item: Url::from_str(&url).map_err(|e| ShellError::IncorrectValue {
            msg: format!("Invalid Url: {e}"),
            val_span: spanned_path.span,
            call_span,
        })?,
        span: spanned_path.span,
    };

    let (object_store, path) = plugin.parse_url(engine, &url, call_span).await?;
    let list_stream = object_store.object_store().list(Some(&path));

    let values: Vec<Value> = list_stream
        .map(|v| match v {
            Ok(meta) => Value::record(
                record!(
                    "name" => Value::string(meta.location.to_string(), call_span),
                    "size" => Value::filesize(meta.size as i64, call_span),
                    "modified" => Value::date(meta.last_modified.fixed_offset(), call_span),
                    "etag" => meta.e_tag.map(|s| Value::string(s, call_span)).unwrap_or(Value::nothing(call_span)),
                    "version" => meta.version.map(|s| Value::string(s, call_span)).unwrap_or(Value::nothing(call_span)),
                ),
                call_span,
            ),
            Err(e) => {
                let se = ShellError::GenericError {
                    error: format!("Error fetching data from object store: {e}"),
                    msg: "".into(),
                    span: None,
                    help: None,
                    inner: vec![],
                };
                Value::error(se, call_span)
            }
        })
        .collect()
        .await;

    Ok(PipelineData::Value(Value::list(values, call_span), None))
}

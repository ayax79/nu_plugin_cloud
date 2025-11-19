use std::{path::PathBuf, str::FromStr, vec};

use nu_plugin::{EngineInterface, PluginCommand};
use nu_protocol::{
    Category, Example, LabeledError, PipelineData, ShellError, Signature, Spanned, SyntaxShape,
    Type,
};
use url::Url;

use crate::CloudPlugin;

pub struct Remove;

impl PluginCommand for Remove {
    type Plugin = CloudPlugin;

    fn name(&self) -> &str {
        "cloud rm"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cloud rm")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .required("uri", SyntaxShape::String, "The file url to use.")
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Remove a file from cloud sotrage"
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Remove a file from s3.",
            example: "cloud rm s3://mybucket/file.txt",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &nu_plugin::EvaluatedCall,
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
    call: &nu_plugin::EvaluatedCall,
) -> Result<PipelineData, ShellError> {
    let call_span = call.head;
    let url_path: Spanned<PathBuf> = call.req(0)?;
    let url = url_path
        .item
        .to_str()
        .expect("The path should already be unicode")
        .to_string();
    let url = Spanned {
        item: Url::from_str(&url).map_err(|e| ShellError::IncorrectValue {
            msg: format!("Invalid Url: {e}"),
            val_span: url_path.span,
            call_span,
        })?,
        span: url_path.span,
    };
    let (object_store, path) = plugin.parse_url(engine, &url, call_span).await?;

    object_store
        .object_store()
        .delete(&path)
        .await
        .map_err(|e| ShellError::GenericError {
            error: format!("Could not delete delete from cloud storage: {e}"),
            msg: "".into(),
            span: Some(call_span),
            help: None,
            inner: vec![],
        })?;

    Ok(PipelineData::empty())
}

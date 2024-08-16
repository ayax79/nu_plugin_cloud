use std::{
    path::PathBuf,
    str::FromStr,
    sync::{atomic::AtomicBool, Arc},
};

use bytes::Buf;
use log::debug;
use nu_plugin::{EngineInterface, PluginCommand};
use nu_protocol::{
    ByteStream, ByteStreamType, Category, DataSource, IntoInterruptiblePipelineData, LabeledError,
    PipelineData, PipelineMetadata, ShellError, Signals, Signature, Spanned, SyntaxShape, Type,
};
use url::Url;

use crate::CloudPlugin;

pub struct Open;

impl PluginCommand for Open {
    type Plugin = CloudPlugin;

    fn name(&self) -> &str {
        "cloud open"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cloud open")
            .input_output_types(vec![(Type::Nothing, Type::Any), (Type::String, Type::Any)])
            .rest("url", SyntaxShape::String, "The cloud url to file to open.")
            .switch("raw", "open file as raw binary", Some('r'))
            .category(Category::FileSystem)
    }

    fn usage(&self) -> &str {
        "Load a file into a cell, converting to table if possible (avoid by appending '--raw')."
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        command(plugin, engine, call, input).map_err(LabeledError::from)
    }
}

fn command(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    _input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let call_span = call.head;
    let raw = call.has_flag("raw")?;
    let spanned_path: Spanned<PathBuf> = call.req(0)?;
    let path = spanned_path.item;
    let url = path
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

    let bytes = plugin.cache_get(&url, call_span)?;

    let content_type = if raw {
        path.extension()
            .map(|ext| ext.to_string_lossy().to_string())
            .and_then(|ref s| detect_content_type(s))
    } else {
        None
    };

    let extension: Option<String> = if raw {
        None
    } else {
        path.extension()
            .map(|ext| ext.to_string_lossy().to_string().to_owned())
            .map(|s| s.to_lowercase())
    };

    let converter = if !raw {
        if let Some(ext) = &extension {
            debug!("Attempting to use converter: {ext}");
            engine.find_decl(format!("from {}", ext))?
        } else {
            None
        }
    } else {
        None
    };

    // todo - once the plugin ctrl-c stuff has been released, integrate it
    let signals = Signals::new(Arc::new(AtomicBool::new(false)));

    let stream = PipelineData::ByteStream(
        ByteStream::read(
            bytes.reader(),
            call_span,
            signals.clone(),
            ByteStreamType::Unknown,
        ),
        Some(PipelineMetadata {
            data_source: DataSource::FilePath(path.to_path_buf()),
            content_type,
        }),
    );

    // since we actually found content, turn off plugin GC so the cache works
    engine.set_gc_disabled(true)?;

    match converter {
        Some(converter_id) => {
            debug!("converter id: {converter_id}");
            let command_output =
                engine.call_decl(converter_id, call.clone(), stream, true, false)?;
            Ok(command_output.into_pipeline_data_with_metadata(
                call.head,
                signals,
                PipelineMetadata::default()
                    .with_data_source(DataSource::FilePath(path.to_path_buf())),
            ))
        }
        None => Ok(stream),
    }
}

fn detect_content_type(extension: &str) -> Option<String> {
    // This will allow the overriding of metadata to be consistent with
    // the content type
    match extension {
        // Per RFC-9512, application/yaml should be used
        "yaml" | "yml" => Some("application/yaml".to_string()),
        _ => mime_guess::from_ext(extension)
            .first()
            .map(|mime| mime.to_string()),
    }
}

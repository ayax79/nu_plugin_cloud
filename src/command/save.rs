use std::{
    io::{ErrorKind, Read},
    path::PathBuf,
    str::FromStr,
    vec,
};

use bytes::Bytes;
use log::debug;
use nu_plugin::{EngineInterface, EvaluatedCall, PluginCommand};
use nu_protocol::{
    process::ChildPipe, shell_error::io::IoError, ByteStreamSource, Category, Example,
    LabeledError, ListStream, PipelineData, ShellError, Signals, Signature, Span, Spanned,
    SyntaxShape, Type, Value,
};
use object_store::{PutPayload, WriteMultipart};
use url::Url;

use crate::CloudPlugin;

pub struct Save;

impl PluginCommand for Save {
    type Plugin = CloudPlugin;

    fn name(&self) -> &str {
        "cloud save"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cloud save")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .required("uri", SyntaxShape::String, "The file url to use.")
            .switch("raw", "save file as raw binary", Some('r'))
            .category(Category::FileSystem)
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Save a csv file to s3.",
            example: "[[a b]; [1 1] [1 2] [2 1] [2 2] [3 1] [3 2]] | to csv | cloud save s3://mybucket/file.csv",
            result: None,
        }]
    }

    fn description(&self) -> &str {
        "Save a file to cloud storage"
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(command(plugin, engine, call, input))
            .map_err(LabeledError::from)
    }
}

async fn command(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
    call: &nu_plugin::EvaluatedCall,
    input: PipelineData,
) -> Result<PipelineData, ShellError> {
    let raw = call.has_flag("raw")?;
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

    match input {
        PipelineData::ByteStream(stream, _metadata) => {
            debug!("Handling byte stream");

            match stream.into_source() {
                ByteStreamSource::Read(read) => {
                    bytestream_to_cloud(plugin, engine, read, &url, call_span).await?;
                }
                ByteStreamSource::File(source) => {
                    bytestream_to_cloud(plugin, engine, source, &url, call_span).await?;
                }
                ByteStreamSource::Child(mut child) => {
                    if let Some(stdout) = child.stdout.take() {
                        let res = match stdout {
                            ChildPipe::Pipe(pipe) => {
                                bytestream_to_cloud(plugin, engine, pipe, &url, call_span).await
                            }
                            ChildPipe::Tee(tee) => {
                                bytestream_to_cloud(plugin, engine, tee, &url, call_span).await
                            }
                        };
                        res?;
                    };
                }
            }

            Ok(PipelineData::Empty)
        }
        PipelineData::ListStream(ls, _pipeline_metadata) if raw => {
            debug!("Handling list stream");
            liststream_to_cloud(plugin, engine, ls, &url, call_span).await?;
            Ok(PipelineData::empty())
        }
        input => {
            debug!("Handling input");
            let bytes = input_to_bytes(input, &url_path.item, raw, engine, call, call_span)?;
            stream_bytes(plugin, engine, bytes, &url, call_span).await?;
            Ok(PipelineData::empty())
        }
    }
}

async fn liststream_to_cloud(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
    ls: ListStream,
    url: &Spanned<Url>,
    span: Span,
) -> Result<(), ShellError> {
    let signals = engine.signals();
    let (object_store, path) = plugin.parse_url(engine, url, span).await?;
    let upload = object_store
        .object_store()
        .put_multipart(&path)
        .await
        .unwrap();
    let mut write = WriteMultipart::new(upload);

    for v in ls {
        signals.check(span)?;
        let bytes = value_to_bytes(v)?;
        write.write(&bytes)
    }

    let _ = write.finish().await.map_err(|e| ShellError::GenericError {
        error: format!("Could not write to S3: {e}"),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
    })?;

    Ok(())
}

async fn bytestream_to_cloud(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
    source: impl Read,
    url: &Spanned<Url>,
    span: Span,
) -> Result<(), ShellError> {
    stream_to_cloud_async(plugin, engine, source, url, span).await
}

async fn stream_to_cloud_async(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
    source: impl Read,
    url: &Spanned<Url>,
    span: Span,
) -> Result<(), ShellError> {
    let signals = engine.signals();
    let (object_store, path) = plugin.parse_url(engine, url, span).await?;
    let upload = object_store
        .object_store()
        .put_multipart(&path)
        .await
        .unwrap();
    let mut write = WriteMultipart::new(upload);

    let _ = generic_copy(source, &mut write, span, signals)?;

    let _ = write.finish().await.map_err(|e| ShellError::GenericError {
        error: format!("Could not write to S3: {e}"),
        msg: "".into(),
        span: None,
        help: None,
        inner: vec![],
    })?;

    Ok(())
}

const DEFAULT_BUF_SIZE: usize = 8192;

// Copied from [`std::io::copy`]
fn generic_copy(
    mut reader: impl Read,
    writer: &mut WriteMultipart,
    span: Span,
    signals: &Signals,
) -> Result<u64, ShellError> {
    let buf = &mut [0; DEFAULT_BUF_SIZE];
    let mut len = 0;
    loop {
        signals.check(span)?;
        let n = match reader.read(buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(ShellError::Io(IoError::new(e.kind(), span, None))),
        };
        len += n;
        writer.write(&buf[..n]);
    }
    Ok(len as u64)
}

/// Convert [`Value::String`] [`Value::Binary`] or [`Value::List`] into [`Vec`] of bytes
///
/// Propagates [`Value::Error`] and creates error otherwise
fn value_to_bytes(value: Value) -> Result<Vec<u8>, ShellError> {
    match value {
        Value::String { val, .. } => Ok(val.into_bytes()),
        Value::Binary { val, .. } => Ok(val),
        Value::List { vals, .. } => {
            let val = vals
                .into_iter()
                .map(Value::coerce_into_string)
                .collect::<Result<Vec<String>, ShellError>>()?
                .join("\n")
                + "\n";

            Ok(val.into_bytes())
        }
        // Propagate errors by explicitly matching them before the final case.
        Value::Error { error, .. } => Err(*error),
        other => Ok(other.coerce_into_string()?.into_bytes()),
    }
}

/// Convert [`PipelineData`] bytes to write in file, possibly converting
/// to format of output file
fn input_to_bytes(
    input: PipelineData,
    path: &std::path::Path,
    raw: bool,
    engine: &EngineInterface,
    call: &EvaluatedCall,
    span: Span,
) -> Result<Vec<u8>, ShellError> {
    let ext = if raw {
        None
    } else if let PipelineData::ByteStream(..) = input {
        None
    } else if let PipelineData::Value(Value::String { .. }, ..) = input {
        None
    } else {
        path.extension()
            .map(|name| name.to_string_lossy().to_string())
    };

    let input = if let Some(ext) = ext {
        convert_to_extension(engine, &ext, input, call)?
    } else {
        input
    };

    value_to_bytes(input.into_value(span)?)
}

/// Convert given data into content of file of specified extension if
/// corresponding `to` command exists. Otherwise attempt to convert
/// data to bytes as is
fn convert_to_extension(
    engine: &EngineInterface,
    extension: &str,
    input: PipelineData,
    call: &EvaluatedCall,
) -> Result<PipelineData, ShellError> {
    if let Some(decl_id) = engine.find_decl(format!("to {extension}"))? {
        debug!("Found to {extension} decl: converting input");
        let command_output = engine.call_decl(decl_id, call.clone(), input, true, false)?;
        Ok(command_output)
    } else {
        Ok(input)
    }
}

async fn stream_bytes(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
    bytes: Vec<u8>,
    url: &Spanned<Url>,
    span: Span,
) -> Result<(), ShellError> {
    let (object_store, path) = plugin.parse_url(engine, url, span).await?;

    let payload = PutPayload::from_bytes(Bytes::from(bytes));
    object_store
        .object_store()
        .put(&path, payload)
        .await
        .map_err(|e| ShellError::GenericError {
            error: format!("Could not write to S3: {e}"),
            msg: "".into(),
            span: None,
            help: None,
            inner: vec![],
        })?;

    Ok(())
}

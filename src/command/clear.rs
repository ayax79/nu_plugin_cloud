use crate::CloudPlugin;
use nu_plugin::{EngineInterface, PluginCommand};
use nu_protocol::{Category, Example, LabeledError, PipelineData, ShellError, Signature, Type};

pub struct Clear;

impl PluginCommand for Clear {
    type Plugin = CloudPlugin;

    fn name(&self) -> &str {
        "cloud cache-clear"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cloud cache-clear")
            .input_output_types(vec![(Type::Any, Type::Nothing)])
            .category(Category::FileSystem)
    }

    fn description(&self) -> &str {
        "Clears plugin internal caches. This will also re-enable plugin GC."
    }

    fn examples(&self) -> Vec<Example> {
        vec![Example {
            description: "Clear plugin cache",
            example: "cloud cache-clear",
            result: None,
        }]
    }

    fn run(
        &self,
        plugin: &Self::Plugin,
        engine: &EngineInterface,
        _call: &nu_plugin::EvaluatedCall,
        _input: PipelineData,
    ) -> Result<PipelineData, LabeledError> {
        plugin
            .rt
            .block_on(command(plugin, engine))
            .map_err(LabeledError::from)
    }
}

async fn command(
    plugin: &CloudPlugin,
    engine: &EngineInterface,
) -> Result<PipelineData, ShellError> {
    plugin.cache.clear(engine).await?;
    Ok(PipelineData::empty())
}

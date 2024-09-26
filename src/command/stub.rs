use nu_plugin::PluginCommand;
use nu_protocol::{Category, PipelineData, Signature, Type, Value};

use crate::CloudPlugin;

pub struct Stub;

impl PluginCommand for Stub {
    type Plugin = CloudPlugin;

    fn name(&self) -> &str {
        "cloud"
    }

    fn signature(&self) -> nu_protocol::Signature {
        Signature::build("cloud")
            .category(Category::FileSystem)
            .input_output_types(vec![(Type::Nothing, Type::String)])
    }

    fn description(&self) -> &str {
        "Provides the ability to read and write files from cloud storage"
    }

    fn run(
        &self,
        _plugin: &Self::Plugin,
        engine: &nu_plugin::EngineInterface,
        call: &nu_plugin::EvaluatedCall,
        _input: nu_protocol::PipelineData,
    ) -> Result<nu_protocol::PipelineData, nu_protocol::LabeledError> {
        Ok(PipelineData::Value(
            Value::string(engine.get_help()?, call.head),
            None,
        ))
    }
}

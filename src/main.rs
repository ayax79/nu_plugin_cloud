use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_cloud::CloudPlugin;

fn main() {
    env_logger::init();
    serve_plugin(&CloudPlugin::default(), MsgPackSerializer {})
}

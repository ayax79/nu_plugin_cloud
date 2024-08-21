use log::debug;
use nu_plugin::{serve_plugin, MsgPackSerializer};
use nu_plugin_cloud::CloudPlugin;

fn main() {
    env_logger::init();
    debug!("Starting cloud plugin");
    serve_plugin(&CloudPlugin::default(), MsgPackSerializer {})
}

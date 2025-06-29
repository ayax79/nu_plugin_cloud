use log::debug;
use nu_plugin::{MsgPackSerializer, serve_plugin};
use nu_plugin_cloud::CloudPlugin;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    env_logger::init();
    debug!("Starting cloud plugin");
    serve_plugin(&CloudPlugin::default(), MsgPackSerializer {})
}

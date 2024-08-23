use crate::CloudPlugin;

mod clear;
mod ls;
mod open;
mod rm;
mod save;
mod stub;

pub fn commands() -> Vec<Box<dyn nu_plugin::PluginCommand<Plugin = CloudPlugin>>> {
    vec![
        Box::new(clear::Clear),
        Box::new(ls::Ls),
        Box::new(open::Open),
        Box::new(rm::Remove),
        Box::new(save::Save),
        Box::new(stub::Stub),
    ]
}

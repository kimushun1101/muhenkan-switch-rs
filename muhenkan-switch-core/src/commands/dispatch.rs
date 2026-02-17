use anyhow::Result;

use crate::config::{Config, DispatchAction};

pub fn run(key: &str, config: &Config) -> Result<()> {
    let action = config
        .dispatch_lookup(key)
        .ok_or_else(|| anyhow::anyhow!("No action bound to key '{}' in config.toml", key))?;

    match action {
        DispatchAction::Search { engine } => super::search::run(&engine, config),
        DispatchAction::OpenFolder { target } => super::open_folder::run(&target, config),
        DispatchAction::SwitchApp { target } => super::switch_app::run(&target, config),
    }
}

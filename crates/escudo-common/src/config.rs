use config::{Config, File};
use serde::de::DeserializeOwned;
use std::path::Path;

pub fn load_config<T: DeserializeOwned>(path: &Path) -> anyhow::Result<T> {
    let config = Config::builder()
        .add_source(File::from(path))
        .add_source(config::Environment::with_prefix("ESCUDO").separator("__"))
        .build()?;

    Ok(config.try_deserialize()?)
}

use serde::Deserialize;

use crate::config::SyncConfig;

#[derive(Deserialize, Default)]
pub struct PartitionConfig {
    pub name: Option<String>,
    pub sync: Option<SyncConfig>,
}

pub fn load(vault_dir: &std::path::Path) -> PartitionConfig {
    let path = vault_dir.join("partition.toml");
    let raw = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(_) => return PartitionConfig::default(),
    };
    toml::from_str(&raw).unwrap_or_default()
}

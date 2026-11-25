use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdaterConfig {
    pub url: String,
    pub auto_update_on_launch: bool,
}

/// Updates the selected configuration file when updater.json enables it.
///
/// A missing updater.json means that updating is not configured and is not an
/// error. Other read, parse, download, and write failures are returned.
pub async fn update_if_requested(
    update_requested: bool,
    config_path: impl AsRef<Path>,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    let updater_file = match fs::read_to_string("updater.json") {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.into()),
    };
    let updater_config: UpdaterConfig = serde_json::from_str(&updater_file)?;

    if !updater_config.auto_update_on_launch && !update_requested {
        return Ok(false);
    }

    let response = reqwest::get(&updater_config.url)
        .await?
        .error_for_status()?;
    let config_contents = response.bytes().await?;
    fs::write(config_path, config_contents)?;
    Ok(true)
}

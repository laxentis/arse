use crate::airport;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub rwy_file: String,
    pub no_factor_wind: Option<u32>,
    pub pref_wind: Option<u32>,
    pub assumed_dir: Option<u32>,
    pub airports: Vec<airport::Airport>,
}

impl Config {
    pub fn read(file: &str) -> Config {
        Self::try_read(file).expect("Unable to read or parse config file")
    }

    pub fn try_read(file: &str) -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
        let cfg_file = fs::read_to_string(file)?;
        Ok(serde_json::from_str(&cfg_file)?)
    }
}

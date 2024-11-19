use std::fs;
use serde::{Deserialize, Serialize};
use crate::airport;

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
        let cfg_file = fs::read_to_string(file).expect("Unable to read config file!");
        let cfg: Config = serde_json::from_str(&cfg_file).expect("Unable to parse config file!");
        cfg
    }
}
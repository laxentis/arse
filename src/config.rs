use crate::airport;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub fn config_path_from_args(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<PathBuf, &'static str> {
    Ok(parse_args(args)?.config_path)
}

pub struct Args {
    pub config_path: PathBuf,
    pub update: bool,
}

pub fn parse_args(
    args: impl IntoIterator<Item = std::ffi::OsString>,
) -> Result<Args, &'static str> {
    let mut config_path = None;
    let mut update = false;

    for arg in args {
        if arg == "--update" {
            update = true;
        } else if config_path.is_none() {
            config_path = Some(PathBuf::from(arg));
        } else {
            return Err("Expected at most one argument: the path to the configuration file");
        }
    }

    Ok(Args {
        config_path: config_path.unwrap_or_else(|| "arse.json".into()),
        update,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub rwy_file: String,
    pub no_factor_wind: Option<u32>,
    pub pref_wind: Option<u32>,
    pub assumed_dir: Option<u32>,
    pub airports: Vec<airport::Airport>,
}

impl Config {
    pub fn try_read(
        file: impl AsRef<Path>,
    ) -> Result<Config, Box<dyn std::error::Error + Send + Sync>> {
        let cfg_file = fs::read_to_string(file)?;
        Ok(serde_json::from_str(&cfg_file)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::ffi::OsString;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn accepts_optional_config_path_argument() {
        assert_eq!(
            config_path_from_args([]).unwrap(),
            PathBuf::from("arse.json")
        );
        assert_eq!(
            config_path_from_args([OsString::from(r"config\my file.json")]).unwrap(),
            PathBuf::from(r"config\my file.json")
        );
        assert!(config_path_from_args([OsString::from("a"), OsString::from("b")]).is_err());
    }

    #[test]
    fn accepts_update_flag_with_or_without_config_path() {
        let args = parse_args([OsString::from("--update")]).unwrap();
        assert!(args.update);
        assert_eq!(args.config_path, PathBuf::from("arse.json"));

        let args = parse_args([OsString::from("custom.json"), OsString::from("--update")]).unwrap();
        assert!(args.update);
        assert_eq!(args.config_path, PathBuf::from("custom.json"));
    }

    #[test]
    fn reads_config_and_writes_runway_using_relative_and_absolute_paths() {
        let name = format!(
            "arse-path-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let dir = PathBuf::from(name);
        fs::create_dir(&dir).unwrap();
        let relative_config = dir.join("config file.json");
        let relative_rwy = dir.join("sector file.rwy");
        let absolute_config = std::env::current_dir().unwrap().join(&relative_config);
        let absolute_rwy = std::env::current_dir().unwrap().join(&relative_rwy);

        for (config_path, rwy_path) in [
            (&relative_config, &relative_rwy),
            (&absolute_config, &absolute_rwy),
        ] {
            let json = serde_json::json!({"rwy_file": rwy_path, "airports": []});
            fs::write(config_path, json.to_string()).unwrap();
            let config = Config::try_read(config_path).unwrap();
            crate::runway_file::try_write(
                &config.rwy_file,
                HashMap::from([("ZZZZ".to_string(), vec!["36R".to_string()])]),
                HashMap::new(),
            )
            .unwrap();
            assert_eq!(
                fs::read_to_string(rwy_path).unwrap(),
                "ACTIVE_RUNWAY:ZZZZ:36R:1\n"
            );
        }
        fs::remove_file(relative_config).unwrap();
        fs::remove_file(relative_rwy).unwrap();
        fs::remove_dir(dir).unwrap();
    }
}

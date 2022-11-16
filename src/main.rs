use std::fs;
use reqwest::Url;
use serde::{Serialize, Deserialize};
use exitfailure::ExitFailure;

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    sector_file: String,
    airports: Vec<Airport>
}

#[derive(Debug, Serialize, Deserialize)]
struct Runway {
    id: String,
    true_heading: u16
}

#[derive(Debug, Serialize, Deserialize)]
struct Airport {
    icao: String,
    runways: Vec<Runway>,
    use_metar_from: Option<String>
}

impl Airport {
    async fn get_wx(&self) -> Result<String, ExitFailure> {
        let icao: String;
        match &self.use_metar_from {
            Some(s) => { icao = s.to_string()},
            None => {icao = self.icao.clone()}
        }
        let url = format!("https://metar.vatsim.net/metar.php?id={}", icao);
        let url = Url::parse(&*url)?;
        let res = reqwest::get(url).await?.text().await?;
        Ok(res)
    }
}

fn read_config(file: &str) -> Config {
    let cfg_file = fs::read_to_string(file).expect("Unable to read config file!");
    let cfg: Config = serde_json::from_str(&cfg_file).expect("Unable to parse config file!");
    return cfg;
}

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    println!("Automatic Runway Setting for Euroscope");
    print!("Reading config: ");
    let cfg = read_config("arse.json");
    println!("OK");
    for airport in cfg.airports.iter() {
        let wx = airport.get_wx().await?;
        println!("{}: {}", airport.icao, wx);
    }
    Ok(())
}

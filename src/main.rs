mod airport;
mod config;
mod runway_file;

use config::Config;
use exitfailure::ExitFailure;
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    println!("Automatic Runway Setting for Euroscope");
    println!("Version {}", VERSION);
    print!("Reading config: ");
    let cfg = Config::read("arse.json");
    println!("OK");
    let mut departure_runways = HashMap::new();
    let mut arrival_runways = HashMap::new();
    println!("Processing data: ");
    for airport in cfg.airports.iter() {
        print!("{} ", airport.icao);
        let (d, a) = airport
            .select_rwy(cfg.no_factor_wind, cfg.pref_wind, cfg.assumed_dir)
            .await?;
        departure_runways.insert(airport.icao.clone(), d);
        arrival_runways.insert(airport.icao.clone(), a);
    }
    println!("Done");
    print!("Writing file: ");
    runway_file::write(cfg.rwy_file, departure_runways, arrival_runways);
    println!("OK");
    Ok(())
}

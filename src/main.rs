mod airport;
mod config;
mod logging;
mod runway_file;
mod updater;
mod weather;

use config::Config;
use exitfailure::ExitFailure;
use std::collections::HashMap;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    logging::init_cli();
    logging::trace("Automatic Runway Setting for Euroscope");
    logging::trace(format!("Version {VERSION}"));
    let args = config::parse_args(std::env::args_os().skip(1))
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidInput, err))?;
    if updater::update_if_requested(args.update, &args.config_path)
        .await
        .map_err(|err| std::io::Error::other(format!("Could not update configuration: {err}")))?
    {
        logging::trace("Configuration updated");
    }
    logging::trace("Reading config");
    let config_path = args.config_path;
    let cfg = Config::try_read(&config_path).map_err(|err| {
        std::io::Error::other(format!("Could not read {}: {err}", config_path.display()))
    })?;
    logging::trace("Configuration read successfully");
    let mut departure_runways = HashMap::new();
    let mut arrival_runways = HashMap::new();
    logging::trace("Processing data");
    let reports = weather::fetch(&cfg.airports).await?;
    for airport in cfg.airports.iter() {
        logging::trace(format!("Processing {}", airport.icao));
        let (d, a) = airport.select_rwy(
            weather::report(&reports, airport),
            cfg.no_factor_wind,
            cfg.pref_wind,
            cfg.assumed_dir,
        )?;
        departure_runways.insert(airport.icao.clone(), d);
        arrival_runways.insert(airport.icao.clone(), a);
    }
    logging::trace("Processing complete");
    logging::trace("Writing runway file");
    runway_file::write(cfg.rwy_file, departure_runways, arrival_runways);
    logging::trace("Runway file written successfully");
    Ok(())
}

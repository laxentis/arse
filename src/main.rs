use exitfailure::ExitFailure;
use metar::{Metar, WindDirection};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::{fs, collections::HashMap, io::Write, cmp};

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    rwy_file: String,
    no_factor_wind: Option<u32>,
    pref_wind: Option<u32>,
    assumed_dir: Option<u32>,
    airports: Vec<Airport>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Runway {
    id: String,
    true_heading: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Airport {
    icao: String,
    runways: Vec<Runway>,
    use_metar_from: Option<String>,
    preferred_dep: Option<Vec<Runway>>,
    preferred_arr: Option<Vec<Runway>>,
    selected_dep_rwy: Option<String>,
    selected_arr_rwy: Option<String>
}

impl Runway {
    fn get_wind_dir_difference(&self, wind_dir: u32) -> u32 {
        //self.true_heading.abs_diff(wind_dir)
        cmp::min(360 - self.true_heading.abs_diff(wind_dir), self.true_heading.abs_diff(wind_dir))
    }
}

impl Airport {
    async fn select_rwy(&self, calm_wind:Option<u32>, pref_wind:Option<u32>, assumed_dir:Option<u32>) -> Result<(String, String), ExitFailure> {
        let icao: String;
        let calm_wind = calm_wind.unwrap_or(5);
        let pref_calm_wind = pref_wind.unwrap_or(15);
        let assumed_dir = assumed_dir.unwrap_or(270);
        match &self.use_metar_from {
            Some(s) => icao = s.to_string(),
            None => icao = self.icao.clone(),
        }
        let url = format!("https://metar.vatsim.net/metar.php?id={}", icao);
        let url = Url::parse(&*url)?;
        let res = reqwest::get(url).await?.text().await?;
        let metar = Metar::parse(res).unwrap();
        let mut direction: u32;
        match metar.wind.dir.unwrap() {
            WindDirection::Heading(dir) => {
                direction = *dir;
            }
            WindDirection::Variable => {
                direction = assumed_dir;
            }
            WindDirection::Above => {
                direction = assumed_dir;
            }
        }
        let speed: u32;
        match metar.wind.speed.unwrap() {
            metar::WindSpeed::Calm => {
                speed = 0;
            }
            metar::WindSpeed::Knot(s) => speed = *s,
            metar::WindSpeed::MetresPerSecond(s) => speed = *s * 2,
            metar::WindSpeed::KilometresPerHour(s) => speed = *s / 2,
        }
        print!("WIND: {:0>3} {:0>2}kt ", direction, speed);
        let min_wind: u32;
        if self.preferred_arr.is_none() {
            min_wind = calm_wind;
        } else {
            min_wind = pref_calm_wind;
        }
        if speed < min_wind {
            direction = 270; // For winds below 3 knots use preferred runway
        }
        let dep = self.select_dep_rwy(direction).unwrap();
        let arr = self.select_arr_rwy(direction).unwrap();
        println!("DEP: {:<3} ARR: {:<3}", dep, arr);
        Ok((dep, arr))
    }

    fn select_dep_rwy(&self, wind_dir: u32) -> Result<String, ExitFailure> {
        let dep: String;
        if let Some(dep_prefs) = &self.preferred_dep {
            dep = self.select_preferred_rwy(wind_dir, dep_prefs).unwrap();
        } else {
            dep = self.select_any_rwy(wind_dir, &self.runways).unwrap();
        }
        Ok(dep)
    }

    fn select_arr_rwy(&self, wind_dir: u32) -> Result<String, ExitFailure> {
        let dep: String;
        if let Some(arr_prefs) = &self.preferred_arr {
            dep = self.select_preferred_rwy(wind_dir, arr_prefs).unwrap();
        } else {
            dep = self.select_any_rwy(wind_dir, &self.runways).unwrap();
        }
        Ok(dep)
    }

    fn select_preferred_rwy(&self, direction: u32, runway_list:&Vec<Runway>) -> Result<String, ExitFailure>
    {
        for rwy in runway_list.iter() {
            if rwy.get_wind_dir_difference(direction) < 90 {
                return Ok(rwy.id.clone())
            }
        }
        panic!("No runway selected");
    }

    fn select_any_rwy(&self, direction: u32, runway_list:&Vec<Runway>) -> Result<String, ExitFailure> {
        let mut selected: Option<String> = None;
        let mut diff: u32 = 180;
        for rwy in runway_list.iter() {
            let nd = rwy.get_wind_dir_difference(direction);
            if nd < diff {
                diff = nd;
                selected = Some(rwy.id.clone())
            }
        }
        match selected {
            Some(r) => {
                Ok(r)
            },
            None => panic!("Could not select runway")
        }
    }
}

fn read_config(file: &str) -> Config {
    let cfg_file = fs::read_to_string(file).expect("Unable to read config file!");
    let cfg: Config = serde_json::from_str(&cfg_file).expect("Unable to parse config file!");
    return cfg;
}

fn write_runway_file(file: String, dep: HashMap<String, String>, arr: HashMap<String, String>) {
    let mut f = match fs::OpenOptions::new().write(true).truncate(true).create(true).open(file) {
        Ok(it) => it,
        Err(err) => panic!("Couldn't open the file! {}", err),
    };
    for (icao, rwy) in dep {
        let line = format!("ACTIVE_RUNWAY:{}:{}:1\n",icao,rwy);
        match f.write_all(line.as_bytes()) {
            Ok(_) => {},
            Err(err) => panic!("Couldn't write departures to the file the file! {}", err),
        }
    }
    for (icao, rwy) in arr {
        let line = format!("ACTIVE_RUNWAY:{}:{}:0\n",icao,rwy);
        match f.write_all(line.as_bytes()) {
            Ok(_) => {},
            Err(err) => panic!("Couldn't write arrivals to the file the file! {}", err),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), ExitFailure> {
    println!("Automatic Runway Setting for Euroscope");
    print!("Reading config: ");
    let cfg = read_config("arse.json");
    println!("OK");
    let mut dep_rwys = HashMap::new();
    let mut arr_rwys = HashMap::new();
    println!("Processing data: ");
    for airport in cfg.airports.iter() {
        print!("{} ", airport.icao);
        let (d,a) = airport.select_rwy(cfg.no_factor_wind, cfg.pref_wind, cfg.assumed_dir).await?;
        dep_rwys.insert(airport.icao.clone(), d);
        arr_rwys.insert(airport.icao.clone(), a);
    }
    println!("Done");
    print!("Writing file: ");
    write_runway_file(cfg.rwy_file, dep_rwys, arr_rwys);
    println!("OK");
    Ok(())
}

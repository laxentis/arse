use exitfailure::ExitFailure;
use metar::{Metar, WindDirection};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::cmp;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Runway {
    id: String,
    true_heading: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Airport {
    pub icao: String,
    runways: Vec<Runway>,
    use_metar_from: Option<String>,
    preferred_dep: Option<Vec<String>>,
    preferred_arr: Option<Vec<String>>,
    selected_dep_rwy: Option<String>,
    selected_arr_rwy: Option<String>,
    no_factor_wind: Option<u32>,
}

impl Runway {
    fn get_wind_dir_difference(&self, wind_dir: u32) -> u32 {
        //self.true_heading.abs_diff(wind_dir)
        cmp::min(
            360 - self.true_heading.abs_diff(wind_dir),
            self.true_heading.abs_diff(wind_dir),
        )
    }
}

impl Airport {
    fn get_runway_heading(&self, id: String) -> Result<&Runway, String> {
        for runway in &self.runways {
            if runway.id == id {
                return Ok(runway);
            }
        }
        Err("No runway found".to_string())
    }

    pub async fn select_rwy(
        &self,
        calm_wind_prop: Option<u32>,
        pref_wind: Option<u32>,
        assumed_dir: Option<u32>,
    ) -> Result<(String, String), ExitFailure> {
        let icao: String;
        let calm_wind: u32;
        if self.no_factor_wind.is_some() {
            calm_wind = self.no_factor_wind.unwrap();
        } else {
            calm_wind = calm_wind_prop.unwrap_or(5);
        }
        let pref_calm_wind = pref_wind.unwrap_or(15);
        let assumed_dir = assumed_dir.unwrap_or(270);
        match &self.use_metar_from {
            Some(s) => icao = s.to_string(),
            None => icao = self.icao.clone(),
        }
        let url = format!("https://metar.vatsim.net/metar.php?id={}", icao);
        let url = Url::parse(&*url)?;
        let res = reqwest::get(url).await?.text().await?;
        let metar = Metar::parse(res)?;
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
            direction = assumed_dir; // For winds below 3 knots use preferred runway
        }
        let dep = self.select_dep_rwy(direction)?;
        let arr = self.select_arr_rwy(direction)?;
        println!("DEP: {:<3} ARR: {:<3}", dep, arr);
        Ok((dep, arr))
    }

    fn select_dep_rwy(&self, wind_dir: u32) -> Result<String, ExitFailure> {
        let dep: String;
        if let Some(dep_prefs) = &self.preferred_dep {
            dep = self.select_preferred_rwy(wind_dir, dep_prefs)?;
        } else {
            dep = self.select_any_rwy(wind_dir, &self.runways)?;
        }
        Ok(dep)
    }

    fn select_arr_rwy(&self, wind_dir: u32) -> Result<String, ExitFailure> {
        let dep: String;
        if let Some(arr_prefs) = &self.preferred_arr {
            dep = self.select_preferred_rwy(wind_dir, arr_prefs)?;
        } else {
            dep = self.select_any_rwy(wind_dir, &self.runways)?;
        }
        Ok(dep)
    }

    fn select_preferred_rwy(
        &self,
        direction: u32,
        runway_list: &Vec<String>,
    ) -> Result<String, ExitFailure> {
        for rwy in runway_list.iter() {
            let rw = self.get_runway_heading(rwy.to_owned()).unwrap();
            if rw.get_wind_dir_difference(direction) < 90 {
                return Ok(rw.id.clone());
            }
        }
        panic!("No runway selected");
    }

    fn select_any_rwy(
        &self,
        direction: u32,
        runway_list: &Vec<Runway>,
    ) -> Result<String, ExitFailure> {
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
            Some(r) => Ok(r),
            None => panic!("Could not select runway"),
        }
    }
}

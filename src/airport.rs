use exitfailure::ExitFailure;
use metar::{Data, Metar, WindDirection};
use serde::{Deserialize, Serialize};
use std::cmp;

use crate::logging;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Runway {
    id: String,
    true_heading: u32,
    #[serde(default)]
    linked_designators: Vec<String>,
    #[serde(default)]
    blocks: Vec<String>,
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
    pub fn metar_station(&self) -> &str {
        self.use_metar_from.as_deref().unwrap_or(&self.icao)
    }

    fn get_runway_heading(&self, id: &str) -> Result<&Runway, String> {
        for runway in &self.runways {
            if runway.id == id {
                return Ok(runway);
            }
        }
        Err("No runway found".to_string())
    }

    pub fn select_rwy(
        &self,
        report: &str,
        calm_wind_prop: Option<u32>,
        _pref_wind: Option<u32>,
        assumed_dir: Option<u32>,
    ) -> Result<(Vec<String>, Vec<String>), ExitFailure> {
        let calm_wind: u32;
        if self.no_factor_wind.is_some() {
            calm_wind = self.no_factor_wind.unwrap();
        } else {
            calm_wind = calm_wind_prop.unwrap_or(5);
        }
        let assumed_dir = assumed_dir.unwrap_or(270);
        let metar = Metar::parse(report).map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid METAR for {}: {err}", self.metar_station()),
            )
        })?;
        let wind_dir = match &metar.wind.dir {
            Data::Known(dir) => dir,
            Data::Unknown => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "METAR has no wind direction",
                )
                .into())
            }
        };
        let direction = match wind_dir {
            WindDirection::Heading(dir) => *dir,
            WindDirection::Variable | WindDirection::Above => assumed_dir,
        };
        let speed: u32;
        let wind_speed = match &metar.wind.speed {
            Data::Known(speed) => speed,
            Data::Unknown => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "METAR has no wind speed",
                )
                .into())
            }
        };
        match wind_speed {
            metar::WindSpeed::Calm => {
                speed = 0;
            }
            metar::WindSpeed::Knot(s) => speed = *s,
            metar::WindSpeed::MetresPerSecond(s) => speed = *s * 2,
            metar::WindSpeed::KilometresPerHour(s) => speed = *s / 2,
        }
        logging::trace(format!("WIND: {:0>3} {:0>2}kt", direction, speed));
        let ignore_wind_direction = speed < calm_wind;
        let arr = self.select_arr_rwy(direction, ignore_wind_direction)?;
        let blocked_for_departure = self
            .get_runway_heading(&arr)
            .map(|runway| runway.blocks.as_slice())
            .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))?;
        let dep = self.select_dep_rwy(direction, ignore_wind_direction, blocked_for_departure)?;
        let dep = self.with_linked_designators(&dep);
        let arr = self.with_linked_designators(&arr);
        logging::trace(format!("DEP: {} ARR: {}", dep.join(", "), arr.join(", ")));
        Ok((dep, arr))
    }

    fn with_linked_designators(&self, selected: &str) -> Vec<String> {
        let mut result = vec![selected.to_owned()];
        if let Some(runway) = self.runways.iter().find(|runway| runway.id == selected) {
            for designator in &runway.linked_designators {
                if !result.contains(designator) {
                    result.push(designator.clone());
                }
            }
        }
        result
    }

    fn select_dep_rwy(
        &self,
        wind_dir: u32,
        ignore_wind_direction: bool,
        blocked: &[String],
    ) -> Result<String, ExitFailure> {
        let dep: String;
        if let Some(dep_prefs) = &self.preferred_dep {
            dep = self.select_preferred_rwy(wind_dir, dep_prefs, ignore_wind_direction, blocked)?;
        } else {
            dep = self.select_any_rwy(wind_dir, &self.runways, ignore_wind_direction, blocked)?;
        }
        Ok(dep)
    }

    fn select_arr_rwy(
        &self,
        wind_dir: u32,
        ignore_wind_direction: bool,
    ) -> Result<String, ExitFailure> {
        let dep: String;
        if let Some(arr_prefs) = &self.preferred_arr {
            dep = self.select_preferred_rwy(wind_dir, arr_prefs, ignore_wind_direction, &[])?;
        } else {
            dep = self.select_any_rwy(wind_dir, &self.runways, ignore_wind_direction, &[])?;
        }
        Ok(dep)
    }

    fn select_preferred_rwy(
        &self,
        direction: u32,
        runway_list: &[String],
        ignore_wind_direction: bool,
        blocked: &[String],
    ) -> Result<String, ExitFailure> {
        for rwy in runway_list.iter() {
            let rw = self
                .get_runway_heading(rwy)
                .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))?;
            if blocked.iter().any(|blocked_id| blocked_id == &rw.id) {
                continue;
            }
            if ignore_wind_direction || rw.get_wind_dir_difference(direction) < 90 {
                return Ok(rw.id.clone());
            }
        }
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "No preferred runway selected",
        )
        .into())
    }

    fn select_any_rwy(
        &self,
        direction: u32,
        runway_list: &[Runway],
        ignore_wind_direction: bool,
        blocked: &[String],
    ) -> Result<String, ExitFailure> {
        let mut selected: Option<String> = None;
        let mut diff: u32 = 180;
        for rwy in runway_list.iter() {
            if blocked.iter().any(|blocked_id| blocked_id == &rwy.id) {
                continue;
            }
            if ignore_wind_direction {
                return Ok(rwy.id.clone());
            }
            let nd = rwy.get_wind_dir_difference(direction);
            if nd < diff {
                diff = nd;
                selected = Some(rwy.id.clone())
            }
        }
        match selected {
            Some(r) => Ok(r),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Could not select runway",
            )
            .into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn airport(json: &str) -> Airport {
        serde_json::from_str(json).unwrap()
    }

    const REPORT: &str = "ZZZZ 010000Z 35020KT CAVOK 10/05 Q1013";

    #[test]
    fn linked_designators_follow_independent_departure_and_arrival_selections() {
        let airport = airport(
            r#"{
                "icao": "ZZZZ",
                "runways": [
                    {"id":"36R","true_heading":353,"linked_designators":["36L","36R","36L"]},
                    {"id":"36L","true_heading":353,"linked_designators":["36C"]},
                    {"id":"18L","true_heading":173}
                ],
                "preferred_dep":["36R"],
                "preferred_arr":["36L"]
            }"#,
        );
        let (dep, arr) = airport.select_rwy(REPORT, None, None, None).unwrap();
        assert_eq!(dep, ["36R", "36L"]);
        assert_eq!(arr, ["36L", "36C"]);
    }

    #[test]
    fn missing_links_preserve_single_runway_selection() {
        let airport = airport(
            r#"{"icao":"ZZZZ","runways":[
                {"id":"36R","true_heading":353},
                {"id":"18L","true_heading":173}
            ]}"#,
        );
        let (dep, arr) = airport.select_rwy(REPORT, None, None, None).unwrap();
        assert_eq!(dep, ["36R"]);
        assert_eq!(arr, ["36R"]);
    }

    #[test]
    fn links_are_not_reciprocal_or_recursive() {
        let airport = airport(
            r#"{"icao":"ZZZZ","runways":[
                {"id":"36R","true_heading":353,"linked_designators":["36L"]},
                {"id":"36L","true_heading":353,"linked_designators":["36C"]},
                {"id":"36C","true_heading":353}
            ]}"#,
        );
        assert_eq!(airport.with_linked_designators("36R"), ["36R", "36L"]);
        assert_eq!(airport.with_linked_designators("36C"), ["36C"]);
    }

    #[test]
    fn low_wind_uses_preferred_order_without_heading_filter() {
        let airport = airport(
            r#"{
                "icao":"ZZZZ",
                "no_factor_wind":5,
                "runways":[
                    {"id":"18","true_heading":180},
                    {"id":"36","true_heading":360}
                ],
                "preferred_dep":["18","36"],
                "preferred_arr":["18","36"]
            }"#,
        );
        let (dep, arr) = airport
            .select_rwy("ZZZZ 010000Z 35004KT CAVOK 10/05 Q1013", None, None, None)
            .unwrap();
        assert_eq!(dep, ["18"]);
        assert_eq!(arr, ["18"]);
    }

    #[test]
    fn arrival_blocks_are_removed_from_departure_candidates() {
        let airport = airport(
            r#"{
                "icao":"ZZZZ",
                "runways":[
                    {"id":"08","true_heading":80,"blocks":["26"]},
                    {"id":"26","true_heading":260},
                    {"id":"10","true_heading":100}
                ],
                "preferred_dep":["26","10"],
                "preferred_arr":["08"]
            }"#,
        );
        let (dep, arr) = airport
            .select_rwy("ZZZZ 010000Z 09020KT CAVOK 10/05 Q1013", None, None, None)
            .unwrap();
        assert_eq!(arr, ["08"]);
        assert_eq!(dep, ["10"]);
    }
}

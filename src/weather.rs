use crate::airport::Airport;
use std::collections::{BTreeSet, HashMap};
use std::io;

const METAR_URL: &str = "https://metar.vatsim.net/metar";

fn stations(airports: &[Airport]) -> Vec<String> {
    airports
        .iter()
        .map(|airport| airport.metar_station().trim().to_ascii_uppercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn parse_reports(body: &str) -> HashMap<String, String> {
    body.lines()
        .filter_map(|line| {
            let report = line.trim();
            let station = report.split_whitespace().next()?;
            Some((station.to_ascii_uppercase(), report.to_owned()))
        })
        .collect()
}

async fn fetch_from(url: &str, airports: &[Airport]) -> Result<HashMap<String, String>, String> {
    let stations = stations(airports);
    if stations.is_empty() {
        return Ok(HashMap::new());
    }
    let response = reqwest::Client::new()
        .get(url)
        .query(&[("id", stations.join(","))])
        .send()
        .await
        .map_err(|err| format!("Could not fetch METARs: {err}"))?
        .error_for_status()
        .map_err(|err| format!("METAR request failed: {err}"))?;
    let body = response
        .text()
        .await
        .map_err(|err| format!("Could not read METAR response: {err}"))?;
    let reports = parse_reports(&body);
    let missing: Vec<_> = stations
        .iter()
        .filter(|station| !reports.contains_key(*station))
        .cloned()
        .collect();
    if !missing.is_empty() {
        return Err(format!("No METAR returned for: {}", missing.join(", ")));
    }
    Ok(reports)
}

pub async fn fetch(airports: &[Airport]) -> Result<HashMap<String, String>, io::Error> {
    fetch_from(METAR_URL, airports)
        .await
        .map_err(io::Error::other)
}

pub fn report<'a>(reports: &'a HashMap<String, String>, airport: &Airport) -> &'a str {
    // fetch verifies that every requested station is present.
    &reports[&airport.metar_station().trim().to_ascii_uppercase()]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn airports() -> Vec<Airport> {
        serde_json::from_str(
            r#"[
                {"icao":"EPWA","runways":[]},
                {"icao":"EPBC","use_metar_from":"EPWA","runways":[]},
                {"icao":"EDDB","runways":[]},
                {"icao":"EPWA","runways":[]}
            ]"#,
        )
        .unwrap()
    }

    #[test]
    fn collects_unique_stations_including_overrides() {
        assert_eq!(stations(&airports()), vec!["EDDB", "EPWA"]);
    }

    #[tokio::test]
    async fn fetches_all_stations_in_one_request_and_matches_overrides() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/metar", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            let size = stream.read(&mut request).unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(request.starts_with("GET /metar?id=EDDB%2CEPWA "));
            let body =
                "EPWA 010000Z 27005KT CAVOK 10/05 Q1013\nEDDB 010000Z 09005KT CAVOK 10/05 Q1013\n";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let airports = airports();
        let reports = fetch_from(&url, &airports).await.unwrap();
        assert_eq!(
            report(&reports, &airports[0]),
            report(&reports, &airports[1])
        );
        assert!(report(&reports, &airports[2]).starts_with("EDDB "));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn reports_missing_stations() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/metar", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            stream.read(&mut request).unwrap();
            let body = "EPWA 010000Z 27005KT CAVOK 10/05 Q1013\n";
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        assert!(fetch_from(&url, &airports())
            .await
            .unwrap_err()
            .contains("EDDB"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn reports_http_errors_instead_of_parsing_the_error_body() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/metar", listener.local_addr().unwrap());
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0; 4096];
            stream.read(&mut request).unwrap();
            write!(
                stream,
                "HTTP/1.1 429 Too Many Requests\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
        });
        assert!(fetch_from(&url, &airports())
            .await
            .unwrap_err()
            .contains("429"));
        server.join().unwrap();
    }

    #[tokio::test]
    async fn empty_airports_require_no_request() {
        assert!(fetch_from("http://127.0.0.1:1/metar", &[])
            .await
            .unwrap()
            .is_empty());
    }
}

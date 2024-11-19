use std::collections::HashMap;
use std::fs;
use std::io::Write;

pub fn write(file: String, dep: HashMap<String, String>, arr: HashMap<String, String>) {
    let mut f = match fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file)
    {
        Ok(it) => it,
        Err(err) => panic!("Couldn't open the file! {}", err),
    };
    for (icao, rwy) in dep {
        let line = format!("ACTIVE_RUNWAY:{}:{}:1\n", icao, rwy);
        match f.write_all(line.as_bytes()) {
            Ok(_) => {}
            Err(err) => panic!("Couldn't write departures to the file! {}", err),
        }
    }
    for (icao, rwy) in arr {
        let line = format!("ACTIVE_RUNWAY:{}:{}:0\n", icao, rwy);
        match f.write_all(line.as_bytes()) {
            Ok(_) => {}
            Err(err) => panic!("Couldn't write arrivals to the file! {}", err),
        }
    }
}

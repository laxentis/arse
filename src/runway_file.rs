use std::collections::HashMap;
use std::fs;
use std::io::Write;

pub fn write(file: String, dep: HashMap<String, Vec<String>>, arr: HashMap<String, Vec<String>>) {
    try_write(&file, dep, arr).expect("Couldn't write runway file");
}

pub fn try_write(
    file: &str,
    dep: HashMap<String, Vec<String>>,
    arr: HashMap<String, Vec<String>>,
) -> std::io::Result<()> {
    let mut f = fs::OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(file)?;
    write_entries(&mut f, dep, arr)
}

fn write_entries(
    f: &mut impl Write,
    dep: HashMap<String, Vec<String>>,
    arr: HashMap<String, Vec<String>>,
) -> std::io::Result<()> {
    for (icao, runways) in dep {
        for rwy in runways {
            let line = format!("ACTIVE_RUNWAY:{}:{}:1\n", icao, rwy);
            f.write_all(line.as_bytes())?;
        }
    }
    for (icao, runways) in arr {
        for rwy in runways {
            let line = format!("ACTIVE_RUNWAY:{}:{}:0\n", icao, rwy);
            f.write_all(line.as_bytes())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_every_active_designator_for_each_operation() {
        let dep = HashMap::from([(
            "ZZZZ".to_string(),
            vec!["36R".to_string(), "36L".to_string()],
        )]);
        let arr = HashMap::from([(
            "ZZZZ".to_string(),
            vec!["36L".to_string(), "36C".to_string()],
        )]);
        let mut output = Vec::new();
        write_entries(&mut output, dep, arr).unwrap();
        assert_eq!(
            String::from_utf8(output).unwrap(),
            "ACTIVE_RUNWAY:ZZZZ:36R:1\nACTIVE_RUNWAY:ZZZZ:36L:1\n\
             ACTIVE_RUNWAY:ZZZZ:36L:0\nACTIVE_RUNWAY:ZZZZ:36C:0\n"
        );
    }
}

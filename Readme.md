# Automatic Runway Setting for Euroscope

Did you ever forget to preset runways in use for an airport and be surprised by a traffic flying there? The default EuroScope's dialog is no help there with a million of checkboxes right next to each other.

This tool checks VATSIM METARs and preselects the runways for you.

METARs for all configured airports are fetched in one request. Airports sharing
a `use_metar_from` station use the same report.

**Caution**: The tool has to be launched **before** loading the sector file in EuroScope.

## Running

Run the original command-line application with `cargo run`. To use the GPUI
desktop interface, run `cargo run --features gui --bin arse-gui`. Both read
`arse.json` from the current working directory by default. Pass an optional
configuration path after `--` to use a different file, for example:

```powershell
cargo run -- "C:\EuroScope\config\arse.json"
cargo run --features gui --bin arse-gui -- ".\config\arse.json"
```

When running the executable directly, omit Cargo's `--` separator. Both the
configuration path and the `rwy_file` path can be absolute or relative to the
process's current working directory. In JSON, escape Windows backslashes (for
example `"C:\\EuroScope\\sector.rwy"`), or use forward slashes
(`"C:/EuroScope/sector.rwy"`). Quote paths containing spaces on the command line.

In the GUI, click **Select and
write runways** to fetch METARs, select runways, and overwrite the configured
`.rwy` file. Results or errors are shown in the window.

The GUI builds GPUI and its platform dependencies; on Linux these include the
development libraries required by GPUI's X11/Wayland backends.

## Configuration
The default configuration file is `arse.json`. Edit it to your needs.

### Config preamble
`rwy_file` must point to the location of the `.rwy` file (corresponding to your sector file).
The following are optional configuration options:
* `no_factor_wind` - wind, below which the wind direction is ignored for runway assigment
* `pref_wind` - wind, below which the wind direction is ignored when preferred runway list is used
* `assumed_wind_dir` - wind direction that is assumed when it is unknown

### Airport config
`airports` is a list of all processed airports. It has the following required properties:
* `icao` - the ICAO code
* `runways` - a list of runways of the airport 

The following optional properties can be set:
* `use_metar_from` - ICAO of an airport to get the METAR from. Useful for airports that do not have METARS available on VATSIM.
* `preferred_dep` - an ordered list of preferred departure runway ids,
* `preferred_arr` - an ordered list of preferred arrival runway ids,
* `no_factor_wind` - overrides `no_factor_wind` for given airport.

### Runway config
A `runway` consists of following properties:
* `id` - a string identifying a runway, must be the same as the runway ID in sector file (e.g. `"07L"`)
* `true_heading` - true heading of the runway, used to calculate wind angular difference
* `linked_designators` (optional) - a list of additional runway IDs to activate
  whenever this runway is selected. The links apply separately to departures and
  arrivals, are one-way and are not followed recursively. Linked IDs need not
  appear in `runways`, but must match designators in the sector file.

For example, to activate parallel runway 36L whenever 36R is selected:

```json
{"id": "36R", "true_heading": 353, "linked_designators": ["36L"]}
```

To activate 36R when 36L is selected, add a corresponding link to 36L.

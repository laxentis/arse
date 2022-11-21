# Automatic Runway Setting for Euroscope

Did you ever forget to preset runways in use for an airport and be surprised by a traffic flying there? The default EuroScope's dialog is no help there with a million of checkboxes right next to each other.

This tool checks VATSIM METARs and preselects the runways for you.

**Caution**: The tool has to be launched **before** loading the sector file in EuroScope.

## Configuration
The configuration file is located in `arse.json`. Edit it to your needs.

`rwy_file` must point to the location of the `.rwy` file (corresponding to your sector file).
The following are optional configuration options:
* `no_factor_wind` - wind, below which the wind direction is ignored for runway assigment
* `pref_wind` - wind, below which the wind direction is ignored when preferred runway list is used
* `assumed_wind_dir` - wind direction that is asssumed when it is unknown

`airports` is a list of all processed airports. It has the following required properties:
* `icao` - the ICAO code
* `runways` - a list of runways of the airport
The following optional properties can be set:
* `use_metar_from` - ICAO of an airport to get the METAR from. Useful for airports that do not have METARS available on VATSIM.
* `preferred_dep` - an ordered list of preferred departure runways
* `preferred_arr` - an ordered list of preferred arrival runways.

A `runway` consists of following properties:
* `id` - a string identifying a runway, must be the same as the runway ID in sector file (e.g. `"07L"`)
* `true_heading` - true heading of the runway, used to calculate wind angular difference
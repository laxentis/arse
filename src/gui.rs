mod airport;
mod config;
mod runway_file;
mod weather;

use std::collections::HashMap;

use gpui::{div, prelude::*, px, rgb, size, App, Application, Bounds, Context, TitlebarOptions, Window, WindowBounds, WindowOptions, SharedString};

struct ResultRow {
    icao: String,
    departure: String,
    arrival: String,
}

struct RunResult {
    path: String,
    rows: Vec<ResultRow>,
}

fn run_selection() -> Result<RunResult, String> {
    let cfg = config::Config::try_read("arse.json")
        .map_err(|err| format!("Could not read arse.json: {err}"))?;
    let runtime = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    let mut departures = HashMap::new();
    let mut arrivals = HashMap::new();
    let mut rows = Vec::new();
    let reports = runtime
        .block_on(weather::fetch(&cfg.airports))
        .map_err(|err| err.to_string())?;

    for airport in &cfg.airports {
        let (departure, arrival) = airport
            .select_rwy(
                weather::report(&reports, airport),
                cfg.no_factor_wind,
                cfg.pref_wind,
                cfg.assumed_dir,
            )
            .map_err(|err| format!("{}: {err:?}", airport.icao))?;
        departures.insert(airport.icao.clone(), departure.clone());
        arrivals.insert(airport.icao.clone(), arrival.clone());
        rows.push(ResultRow {
            icao: airport.icao.clone(),
            departure: departure.join(", "),
            arrival: arrival.join(", "),
        });
    }
    runway_file::try_write(&cfg.rwy_file, departures, arrivals)
        .map_err(|err| format!("Could not write {}: {err}", cfg.rwy_file))?;
    Ok(RunResult {
        path: cfg.rwy_file,
        rows,
    })
}

struct RunwayApp {
    running: bool,
    status: String,
    rows: Vec<ResultRow>,
}

impl RunwayApp {
    fn start(&mut self, cx: &mut Context<Self>) {
        if self.running {
            return;
        }
        self.running = true;
        self.status = "Fetching METARs and selecting runways…".into();
        self.rows.clear();
        cx.notify();

        let task = cx.background_executor().spawn(async { run_selection() });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.running = false;
                match result {
                    Ok(result) => {
                        this.status = format!("Runway file written: {}", result.path);
                        this.rows = result.rows;
                    }
                    Err(error) => this.status = error,
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }
}

impl Render for RunwayApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let rows = self
            .rows
            .iter()
            .map(|row| {
                div()
                    .flex()
                    .py_2()
                    .border_b_1()
                    .border_color(rgb(0x27354a))
                    .child(div().w_1_3().child(row.icao.clone()))
                    .child(div().w_1_3().child(row.departure.clone()))
                    .child(div().w_1_3().child(row.arrival.clone()))
            })
            .collect::<Vec<_>>();

        div()
            .size_full()
            .flex()
            .flex_col()
            .gap_4()
            .p_6()
            .bg(rgb(0x101827))
            .text_color(rgb(0xe5edf8))
            .child(div().text_2xl().child("Automatic Runway Setting"))
            .child(
                div().text_sm().text_color(rgb(0x9eafc5)).child(
                    "Run before loading the sector file in EuroScope. Configuration: arse.json",
                ),
            )
            .child(
                div()
                    .id("run")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .bg(if self.running {
                        rgb(0x40516a)
                    } else {
                        rgb(0x2563eb)
                    })
                    .cursor_pointer()
                    .child(if self.running {
                        "Working…"
                    } else {
                        "Select and write runways"
                    })
                    .on_click(cx.listener(|this, _, _, cx| this.start(cx))),
            )
            .child(div().text_sm().child(self.status.clone()))
            .child(
                div()
                    .flex()
                    .text_sm()
                    .text_color(rgb(0x9eafc5))
                    .child(div().w_1_3().child("AIRPORT"))
                    .child(div().w_1_3().child("DEPARTURE"))
                    .child(div().w_1_3().child("ARRIVAL")),
            )
            .child(
                div()
                    .id("results")
                    .flex_1()
                    .overflow_y_scroll()
                    .children(rows),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();
        let bounds = Bounds::centered(None, size(px(720.), px(640.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions{
                    title: Some(SharedString::from("ARSE")),
                    traffic_light_position: None,
                    appears_transparent: false,
                }),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|_| RunwayApp {
                    running: false,
                    status: "Ready. Select runways to update the configured .rwy file.".into(),
                    rows: Vec::new(),
                })
            },
        )
        .expect("Could not open the application window");
        cx.activate(true);
    });
}

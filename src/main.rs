#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use csv::ReaderBuilder;
use serde::Deserialize;
use eframe::{egui, App, Frame};
use std::error::Error;
use std::time::{Duration, Instant};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
struct LedCoordinate {
    x_led: f64,
    y_led: f64,
}

#[derive(Debug, Deserialize)]
struct RunRace {
    date: DateTime<Utc>,
    driver_number: u32,
    x_led: f64,
    y_led: f64,
    time_delta: u64,
}

struct PlotApp {
    coordinates: Vec<LedCoordinate>,
    run_race_data: Vec<RunRace>,
    start_time: Instant,
    start_datetime: DateTime<Utc>,
    race_started: bool,
    colors: HashMap<u32, egui::Color32>,
    current_index: usize,
    next_update_time: DateTime<Utc>,
}

impl PlotApp {
    fn new(coordinates: Vec<LedCoordinate>, run_race_data: Vec<RunRace>, colors: HashMap<u32, egui::Color32>) -> Self {
        let mut app = Self {
            coordinates,
            run_race_data,
            start_time: Instant::now(),
            start_datetime: Utc::now(),
            race_started: false,
            colors,
            current_index: 0,
            next_update_time: Utc::now(),
        };
        app.calculate_next_update_time(); // Calculate initial next_update_time
        app
    }

    fn reset(&mut self) {
        self.start_time = Instant::now();
        self.start_datetime = Utc::now();
        self.race_started = false;
        self.current_index = 0;
        self.calculate_next_update_time(); // Calculate next_update_time after reset
    }

    fn calculate_next_update_time(&mut self) {
        if let Some(run_data) = self.run_race_data.get(self.current_index) {
            let mut total_time_delta = 0;
            for data in self.run_race_data.iter().take(self.current_index + 1) {
                total_time_delta += data.time_delta;
            }
            self.next_update_time = self.start_datetime + Duration::from_millis(total_time_delta);
        }
    }

    fn update_race(&mut self) {
        if self.race_started {
            let current_time = Utc::now();

            if current_time >= self.next_update_time {
                self.current_index += 1;
                if self.current_index < self.run_race_data.len() {
                    self.calculate_next_update_time(); // Calculate next update time for the next data point
                }
            }
        }
    }

    fn scale_f64(value: f64, scale: i64) -> i64 {
        (value * scale as f64) as i64
    }
}

impl App for PlotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut Frame) {
        self.update_race();

        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Background, egui::Id::new("my_layer")));

        let (min_x, max_x) = self.coordinates.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
            (min.min(coord.x_led), max.max(coord.x_led))
        });
        let (min_y, max_y) = self.coordinates.iter().fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), coord| {
            (min.min(coord.y_led), max.max(coord.y_led))
        });

        let width = max_x - min_x;
        let height = max_y - min_y;

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.separator();
                if let Some(run_data) = self.run_race_data.get(self.current_index) {
                    let timestamp_str = run_data.date.format("%H:%M:%S%.3f").to_string();
                    ui.label(timestamp_str);
                }
                ui.separator();

                if ui.button("START").clicked() {
                    self.race_started = true;
                    self.start_time = Instant::now();
                    self.start_datetime = Utc::now();
                    self.current_index = 0;
                    self.calculate_next_update_time(); // Calculate next update time when race starts
                }
                if ui.button("STOP").clicked() {
                    self.reset();
                }
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            let mut led_colors: HashMap<(i64, i64), egui::Color32> = HashMap::new();
            let scale_factor = 1_000_000;

            for run_data in self.run_race_data.iter().take(self.current_index) {
                let color = self.colors.get(&run_data.driver_number).copied().unwrap_or(egui::Color32::WHITE);

                let coord_key = (
                    Self::scale_f64(run_data.x_led, scale_factor),
                    Self::scale_f64(run_data.y_led, scale_factor),
                );

                led_colors.insert(coord_key, color);
            }

            for coord in &self.coordinates {
                let norm_x = ((coord.x_led - min_x) / width) as f32 * ui.available_width();
                let norm_y = ui.available_height() - (((coord.y_led - min_y) / height) as f32 * ui.available_height());

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x, norm_y),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    egui::Color32::BLACK,
                );
            }

            for ((x, y), color) in led_colors {
                let norm_x = ((x as f64 / scale_factor as f64 - min_x) / width) as f32 * ui.available_width();
                let norm_y = ui.available_height() - (((y as f64 / scale_factor as f64 - min_y) / height) as f32 * ui.available_height());

                painter.rect_filled(
                    egui::Rect::from_min_size(
                        egui::pos2(norm_x, norm_y),
                        egui::vec2(20.0, 20.0),
                    ),
                    egui::Rounding::same(0.0),
                    color,
                );
            }
        });

        ctx.request_repaint();
    }
}

fn main() -> eframe::Result<()> {
    let coordinates = read_coordinates("led_coords.csv").expect("Error reading CSV");

    let run_race_data = read_race_data("master_track_data_with_time_deltas.csv").expect("Error reading CSV");

    let mut colors = HashMap::new();

    colors.insert(1, egui::Color32::from_rgb(30, 65, 255));  // Max Verstappen, Red Bull
    colors.insert(2, egui::Color32::from_rgb(0, 82, 255));   // Logan Sargeant, Williams
    colors.insert(4, egui::Color32::from_rgb(255, 135, 0));  // Lando Norris, McLaren
    colors.insert(10, egui::Color32::from_rgb(2, 144, 240)); // Pierre Gasly, Alpine
    colors.insert(11, egui::Color32::from_rgb(30, 65, 255)); // Sergio Perez, Red Bull
    colors.insert(14, egui::Color32::from_rgb(0, 110, 120)); // Fernando Alonso, Aston Martin
    colors.insert(16, egui::Color32::from_rgb(220, 0, 0));   // Charles Leclerc, Ferrari
    colors.insert(18, egui::Color32::from_rgb(0, 110, 120)); // Lance Stroll, Aston Martin
    colors.insert(20, egui::Color32::from_rgb(160, 207, 205)); // Kevin Magnussen, Haas
    colors.insert(22, egui::Color32::from_rgb(60, 130, 200)); // Yuki Tsunoda, AlphaTauri
    colors.insert(23, egui::Color32::from_rgb(0, 82, 255));  // Alex Albon, Williams
    colors.insert(24, egui::Color32::from_rgb(165, 160, 155)); // Zhou Guanyu, Stake F1
    colors.insert(27, egui::Color32::from_rgb(160, 207, 205)); // Nico Hulkenberg, Haas
    colors.insert(31, egui::Color32::from_rgb(2, 144, 240));   // Esteban Ocon, Alpine
    colors.insert(40, egui::Color32::from_rgb(60, 130, 200));  // Liam Lawson, AlphaTauri
    colors.insert(44, egui::Color32::from_rgb(0, 210, 190));   // Lewis Hamilton, Mercedes
    colors.insert(55, egui::Color32::from_rgb(220, 0, 0));     // Carlos Sainz, Ferrari
    colors.insert(63, egui::Color32::from_rgb(0, 210, 190));   // George Russell, Mercedes
    colors.insert(77, egui::Color32::from_rgb(165, 160, 155)); // Valtteri Bottas, Stake F1
    colors.insert(81, egui::Color32::from_rgb(255, 135, 0));   // Oscar Piastri, McLaren

    let app = PlotApp::new(coordinates, run_race_data, colors);

    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "F1-LED-CIRCUIT SIMULATION",
        native_options,
        Box::new(|_cc| Box::new(app)),
    )
}

fn read_coordinates(file_path: &str) -> Result<Vec<LedCoordinate>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().from_path(file_path)?;
    let mut coordinates = Vec::new();
    for result in rdr.deserialize() {
        let record: LedCoordinate = result?;
        coordinates.push(record);
    }
    Ok(coordinates)
}

fn read_race_data(file_path: &str) -> Result<Vec<RunRace>, Box<dyn Error>> {
    let mut rdr = ReaderBuilder::new().from_path(file_path)?;
    let mut run_race_data = Vec::new();
    for result in rdr.deserialize() {
        let record: RunRace = result?;
        run_race_data.push(record);
    }
    Ok(run_race_data)
}

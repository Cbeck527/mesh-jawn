use super::super::replies::CommandResponse;
use super::CommandMetadata;
use crate::radio::packet_metadata::PacketMetadata;
use anyhow::Result;
use chrono::prelude::*;
use serde::Deserialize;
use std::time::Duration;
use ureq::{Agent, Error as UreqError};

pub static INFO_COMMAND: CommandMetadata = CommandMetadata {
    name: "info",
    aliases: &["status"],
    description: "Current information",
    handler: info_command,
};

fn info_command(
    _coords: &PacketMetadata,
    _argument: &str,
    tracker: &crate::bot::node_tracker::NodeTracker,
) -> Result<CommandResponse> {
    let now: DateTime<Local> = Local::now();
    let nodes = tracker.total_nodes();
    let online = tracker.online_nodes();

    let (conditions, temp) = match fetch_weather() {
        Ok(report) => (report.conditions, report.temp),
        Err(err) => {
            log_weather_error(&err);
            ("unavailable".to_string(), "unavailable".to_string())
        }
    };

    let info = format!(
        "🕰️ It's currently {} on {}
📟 I see {} nodes online ({} total)
🌤️ Current conditions: {}, {} (.weather for forecast)",
        now.format("%T"),
        now.format("%a %b %e %Y"),
        online,
        nodes,
        conditions,
        temp,
    );

    Ok(info.into())
}

const SHORT_FORECAST_API_ENDPOINT: &str = concat!(
    "https://api.open-meteo.com/v1/forecast?",
    "latitude=39.9524&longitude=-75.1636&timezone=America%2FNew_York",
    "&current=temperature_2m,weather_code",
    "&wind_speed_unit=mph&temperature_unit=fahrenheit&precipitation_unit=inch",
);

#[derive(Deserialize)]
struct WeatherAPIResponse {
    current_units: CurrentUnits,
    current: CurrentConditions,
}

#[derive(Deserialize)]
struct CurrentConditions {
    temperature_2m: f64,
    weather_code: i32,
}

#[derive(Deserialize)]
struct CurrentUnits {
    temperature_2m: String,
}

struct WeatherReport {
    temp: String,
    conditions: String,
}

#[derive(Debug)]
enum WeatherError {
    Request(String),
    Status(u16),
}

fn fetch_weather() -> Result<WeatherReport, WeatherError> {
    let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    let agent: Agent = Agent::new_with_config(
        Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(10)))
            .user_agent(user_agent)
            .build(),
    );

    let mut response = agent
        .get(SHORT_FORECAST_API_ENDPOINT)
        .call()
        .map_err(|err| match err {
            UreqError::StatusCode(code) => WeatherError::Status(code),
            other => WeatherError::Request(other.to_string()),
        })?;

    let payload: WeatherAPIResponse = response
        .body_mut()
        .read_json()
        .map_err(|err| WeatherError::Request(err.to_string()))?;

    let WeatherAPIResponse {
        current_units,
        current,
    } = payload;

    let temp = format!(
        "{:.1}{}",
        current.temperature_2m, current_units.temperature_2m
    );
    let conditions = lookup_wmo_code(current.weather_code).to_string();

    Ok(WeatherReport { temp, conditions })
}

fn lookup_wmo_code(code: i32) -> &'static str {
    match code {
        0 => "Clear sky",
        1 => "Mainly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 => "Fog",
        48 => "Depositing rime fog",
        51 => "Light drizzle",
        53 => "Moderate drizzle",
        55 => "Dense drizzle",
        56 => "Light freezing drizzle",
        57 => "Dense freezing drizzle",
        61 => "Slight rain",
        63 => "Moderate rain",
        65 => "Heavy rain",
        66 => "Light freezing rain",
        67 => "Heavy freezing rain",
        71 => "Slight snow fall",
        73 => "Moderate snow fall",
        75 => "Heavy snow fall",
        77 => "Snow grains",
        80 => "Slight rain showers",
        81 => "Moderate rain showers",
        82 => "Violent rain showers",
        85 => "Slight snow showers",
        86 => "Heavy snow showers",
        95 => "Thunderstorm",
        96 => "Thunderstorm with light hail",
        99 => "Thunderstorm with heavy hail",
        _ => "Unknown conditions",
    }
}

fn log_weather_error(err: &WeatherError) {
    match err {
        WeatherError::Request(details) => {
            tracing::error!("Weather fetch failed: {}", details);
        }
        WeatherError::Status(code) => {
            tracing::error!("Weather fetch failed: HTTP status {}", code);
        }
    }
}

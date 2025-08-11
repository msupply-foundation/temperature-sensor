use chrono::{Duration, NaiveDateTime};
use serde_json::{json, Value};

use std::fs::File;
use std::io;
use std::io::BufRead;
use std::path::Path;

use crate::common::{Sensor, SensorType, TemperatureLog};

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn read_sensor_to_json(file_path: &str) -> Value {
    let mut current_json = json!({});
    let mut data_timestamps: Vec<Value> = Vec::new();
    let mut data_temperatures: Vec<Value> = Vec::new();
    let mut json_tag; // = String::new();
    let mut json_value; // = "";

    if let Ok(lines) = read_lines(file_path) {
        for line in lines {
            if let Ok(contents) = line {
                let elements: Vec<&str> = contents.split(",").collect();

                if elements.len() == 2 {
                    // Config/status
                    json_tag = elements[0].to_string();
                    json_value = elements[1];
                    current_json[json_tag] = json_value.into();
                }

                if elements.len() > 3 {
                    // Temperature logs
                    let json_timestamp = format!("{} {}", elements[1], elements[2]); // concatenate date & time
                                                                                     // timestamp & temperature columns expected
                    data_timestamps.push(json_timestamp.into());
                    data_temperatures.push(elements[3].into());
                }
            }
        }
    }

    // Add in temperature log data

    if data_timestamps.len() > 1 {
        data_timestamps.remove(0); // remove first element as it is a header
        current_json["Data"]["Timestamp"] = Value::Array(data_timestamps);
    }
    if data_temperatures.len() > 1 {
        data_temperatures.remove(0); // remove first element as it is a header
        current_json["Data"]["Temperature"] = Value::Array(data_temperatures);
    }

    current_json
}

fn parse_string(json_str: &Value) -> String {
    json_str.to_string().replace("\"", "")
}

fn parse_timestamp(json_str: &Value) -> Option<NaiveDateTime> {
    let parsed_string = parse_string(json_str);
    NaiveDateTime::parse_from_str(&parsed_string, "%Y-%m-%d %H:%M:%S").ok()
}

fn parse_float(json_str: &Value) -> Option<f64> {
    let parsed_string = parse_string(json_str);
    parsed_string.parse::<f64>().ok()
}

fn parse_duration(json_str: &Value) -> Option<Duration> {
    // in seconds

    let parsed_string = parse_string(json_str);

    if let Some(_index) = parsed_string.find(" seconds") {
        if let Some(seconds) = parsed_string.replace(" seconds", "").parse::<i64>().ok() {
            Some(Duration::seconds(seconds))
        } else {
            None
        }
    } else {
        None
    }
}

fn parse_logs(json_str: &Value) -> Option<Vec<TemperatureLog>> {
    let mut logs: Vec<TemperatureLog> = Vec::new();
    let mut log_index = 0;

    loop {
        // loop over logs in Data section until no longer valid
        let json_log = &json_str["Data"];

        if json_log["Temperature"][log_index].is_null() {
            break;
        } else {
            if let Some(log_timestamp) = parse_timestamp(&json_log["Timestamp"][log_index]) {
                // timestamp
                if let Some(log_temperature) = parse_float(&json_log["Temperature"][log_index]) {
                    // temperature
                    logs.push(TemperatureLog {
                        timestamp: log_timestamp,
                        temperature: log_temperature,
                    })
                }
            }
            log_index = log_index + 1;
        }
    }

    if logs.len() > 0 {
        logs.sort_unstable_by_key(|logs| (logs.timestamp));
        Some(logs)
    } else {
        None
    }
}

/// Reads sensor data from the specified sensor CSV file.
pub fn read_sensor_from_file(file_path: &str) -> Option<Sensor> {
    if Path::new(file_path).exists() {
        let file_as_json = read_sensor_to_json(file_path);

        let sensor = Sensor {
            sensor_type: SensorType::LogTag,
            serial: parse_string(&file_as_json["Serial #"]),
            name: parse_string(&file_as_json["Description"]),
            last_connected_timestamp: parse_timestamp(&file_as_json["Last reading"]),
            log_interval: parse_duration(&file_as_json["Reading interval"]),
            breaches: None,
            configs: None,
            logs: parse_logs(&file_as_json),
        };

        Some(sensor)
    } else {
        log::error!("File not found: {}", file_path);
        None
    }
}

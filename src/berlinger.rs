use chrono::{Duration, NaiveDate, NaiveDateTime, NaiveTime};
use serde_json::{json, Value};
use std::fs::File;
use std::io;
use std::fs;
use std::io::{BufRead, Write};
use std::path::Path;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use rs_drivelist::drive_list;

use crate::common::{
    BreachType, Sensor, SensorType, TemperatureBreach, TemperatureBreachConfig, TemperatureLog,
};

#[derive(Debug)]
enum SensorSubType {
    FridgeTag,
    QTag,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn count_whitespace_at_start(input: &str) -> usize {
    input
        .chars()
        .take_while(|ch| ch.is_whitespace() && *ch != '\n')
        .count()
}

fn read_sensor_to_json(file_path: &str) -> Value {
    let mut current_json = json!({});
    let mut data_timestamps: Vec<Value> = Vec::new();
    let mut data_temperatures: Vec<Value> = Vec::new();
    let mut data_breaches: Vec<Value> = Vec::new();
    let mut marker_timestamps: Vec<Value> = Vec::new();
    let mut marker_temperatures: Vec<Value> = Vec::new();
    let mut marker_numbers: Vec<Value> = Vec::new();
    let mut breach_start_timestamps: Vec<Value> = Vec::new();
    let mut breach_end_timestamps: Vec<Value> = Vec::new();
    let mut breach_durations: Vec<Value> = Vec::new();
    let mut breach_temperatures: Vec<Value> = Vec::new();
    let mut breach_timestamps: Vec<Value> = Vec::new();
    let mut breach_activation_timestamps: Vec<Value> = Vec::new();
    let mut level_1 = String::new();
    let mut level_2 = String::new();
    let mut level_3 = String::new();
    let mut level_4 = String::new();
    let mut json_tag; // = String::new();
    let mut json_value; // = "";

    if let Ok(lines) = read_lines(file_path) {
        for line in lines {
            if let Ok(contents) = line {
                let level = count_whitespace_at_start(&contents);
                let elements = contents.split(", ");

                for element in elements {
                    let json_elements: Vec<&str> = element.trim().split(": ").collect();

                    if json_elements[0].len() > 0 {
                        let last_char = json_elements[0].chars().last().unwrap(); // should be safe as we've checked for non-empty element
                        let mut new_level = false;
                        let element_count = json_elements.len();

                        json_tag = json_elements[0].to_string();

                        if last_char == ':' {
                            json_tag.pop(); // remove trailing :

                            if element_count == 1 {
                                // new level if only one element in the line
                                new_level = true;
                            }
                        }

                        if new_level {
                            // start of new level

                            match level {
                                0 => {
                                    level_1 = json_tag.clone();
                                    if level_1 != "Data" && level_1 != "Marker" { // regular format (Data and Marker sections are tab-delimited)
                                        current_json[&level_1] = json!({});
                                    }
                                }
                                1 => {
                                    level_2 = json_tag.clone();
                                    current_json[&level_1][&level_2] = json!({});
                                }
                                2 => {
                                    if level_1 == "Res" && level_2 == "Alarm" {
                                        // QTag can have multiple alarms for the same breach type - initialise here
                                        breach_start_timestamps = Vec::new();
                                        breach_end_timestamps = Vec::new();
                                        breach_durations = Vec::new();
                                        breach_temperatures = Vec::new();
                                        breach_timestamps = Vec::new();
                                        breach_activation_timestamps = Vec::new();
                                    }
                                    level_3 = json_tag.clone();
                                    current_json[&level_1][&level_2][&level_3] = json!({});
                                }
                                3 => {
                                    level_4 = json_tag.clone();
                                    current_json[&level_1][&level_2][&level_3][&level_4] =
                                        json!({});
                                }
                                _ => {}, // do nothing - max level expected is 4
                            }
                        } else {
                            if element_count > 1 {
                                // regular line format

                                json_value = json_elements[1];
                                //println!("Value: {}, Tag: {}", json_value.to_string(), json_tag);

                                match level {
                                    0 => current_json[json_tag] = json_value.into(),
                                    1 => current_json[&level_1][json_tag] = json_value.into(),
                                    2 => {
                                        current_json[&level_1][&level_2][json_tag] =
                                            json_value.into()
                                    }
                                    3 => {
                                        if level_1 == "Res" && level_2 == "Alarm" {
                                            // QTag breach
                                            match json_tag.as_str() {
                                                "TS S" => { // breach start timestamp
                                                    breach_start_timestamps.push(json_value.into());
                                                    current_json[&level_1][&level_2][&level_3]
                                                        [json_tag] = Value::Array(
                                                        breach_start_timestamps.clone(),
                                                    );
                                                }
                                                "TS E" => { // breach end timestamp (optional)
                                                    breach_end_timestamps.push(json_value.into());
                                                    current_json[&level_1][&level_2][&level_3]
                                                        [json_tag] =
                                                        Value::Array(breach_end_timestamps.clone());
                                                }
                                                "t A" => { // breach duration
                                                    breach_durations.push(json_value.into());
                                                    current_json[&level_1][&level_2][&level_3]
                                                        [json_tag] =
                                                        Value::Array(breach_durations.clone());
                                                }
                                                "T M" => { // max/min breach temperature
                                                    breach_temperatures.push(json_value.into());
                                                    current_json[&level_1][&level_2][&level_3]
                                                        [json_tag] =
                                                        Value::Array(breach_temperatures.clone());
                                                }
                                                "TS M" => { //max/min breach timestamp
                                                    breach_timestamps.push(json_value.into());
                                                    current_json[&level_1][&level_2][&level_3]
                                                        [json_tag] =
                                                        Value::Array(breach_timestamps.clone());
                                                }
                                                "TS A" => { // breach activation timestamp
                                                    breach_activation_timestamps
                                                        .push(json_value.into());
                                                    current_json[&level_1][&level_2][&level_3]
                                                        [json_tag] = Value::Array(
                                                        breach_activation_timestamps.clone(),
                                                    );
                                                }
                                                _ => {}, // do nothing - no other tags expected,
                                            }
                                        } else {
                                            current_json[&level_1][&level_2][&level_3][json_tag] =
                                                json_value.into()
                                        }
                                    }
                                    4 => {
                                        current_json[&level_1][&level_2][&level_3][&level_4]
                                            [json_tag] = json_value.into()
                                    }
                                    _ => {}, // do nothing - 4 is maximum level expected
                                }
                            } else {
                                // tab-delimited line format

                                let tab_elements: Vec<&str> = json_tag.split("\t").collect();
                                if level_1 == "Data" { // timestamp & temperature columns expected
                                    data_timestamps.push(tab_elements[0].into());
                                    data_temperatures.push(tab_elements[1].into());

                                    if tab_elements.len() > 2 { // optional breach flag column
                                        data_breaches.push(Value::Bool(true));
                                    } else {
                                        data_breaches.push(Value::Bool(false));
                                    }
                                }
                                if level_1 == "Marker" { // 3 columns expected: index, temperature & timestamp
                                    marker_numbers.push(tab_elements[0].into());
                                    marker_temperatures.push(tab_elements[1].into());
                                    marker_timestamps.push(tab_elements[2].into());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Add in tab-delimited data and markers

    if data_timestamps.len() > 1 {
        data_timestamps.remove(0); // remove first element as it is a header
        current_json["Data"]["Timestamp"] = Value::Array(data_timestamps);
    }
    if data_temperatures.len() > 1 {
        data_temperatures.remove(0); // remove first element as it is a header
        current_json["Data"]["Temperature"] = Value::Array(data_temperatures);
    }
    if data_breaches.len() > 0 {
        data_breaches.remove(0); // remove first element as it is a header
        current_json["Data"]["Breach"] = Value::Array(data_breaches);
    }

    if marker_timestamps.len() > 1 {
        marker_timestamps.remove(0); // remove first element as it is a header
        current_json["Marker"]["Timestamp"] = Value::Array(marker_timestamps);
    }
    if marker_temperatures.len() > 1 {
        marker_temperatures.remove(0); // remove first element as it is a header
        current_json["Marker"]["Temperature"] = Value::Array(marker_temperatures);
    }
    if marker_numbers.len() > 0 {
        marker_numbers.remove(0); // remove first element as it is a header
        current_json["Marker"]["Number"] = Value::Array(marker_numbers);
    }

    current_json
}

fn parse_string(json_str: &Value) -> String {
    json_str.to_string().replace("\"", "")
}

fn parse_timestamp(json_str: &Value) -> Option<NaiveDateTime> {
    let parsed_string = parse_string(json_str);
    NaiveDateTime::parse_from_str(&parsed_string, "%Y-%m-%d %H:%M").ok()
}

fn parse_date(json_str: &Value) -> Option<NaiveDate> {
    let parsed_string = parse_string(json_str);
    NaiveDate::parse_from_str(&parsed_string, "%Y-%m-%d").ok()
}

fn parse_time(json_str: &Value) -> Option<NaiveTime> {
    let parsed_string = parse_string(json_str);
    NaiveTime::parse_from_str(&parsed_string, "%H:%M").ok()
}

fn parse_int(json_str: &Value) -> Option<i64> {
    let parsed_string = parse_string(json_str);
    parsed_string.parse::<i64>().ok()
}

fn parse_float(json_str: &Value) -> Option<f64> {
    let parsed_string = parse_string(json_str);
    parsed_string.parse::<f64>().ok()
}

fn parse_duration(json_str: &Value) -> Option<Duration> {
    // in minutes

    if let Some(minutes) = parse_int(json_str) {
        Some(Duration::minutes(minutes))
    } else {
        None
    }
}

fn parse_subtype(json_str: &Value) -> SensorSubType {
    if json_str["Hist"].is_null() { // Hist section only present for FridgeTag
        SensorSubType::QTag
    } else {
        SensorSubType::FridgeTag
    }
}

fn parse_breach_configs(
    json_str: &Value,
    sensor_subtype: &SensorSubType,
) -> Option<Vec<TemperatureBreachConfig>> {
    let mut breach_configs: Vec<TemperatureBreachConfig> = Vec::new();
    let max_breach_temperature = 100.0; // boiling point of water (should be safe default max!)
    let min_breach_temperature = -273.0; // absolute zero (should be safe default min!)
    let mut max_temperature; // = 0.0;
    let mut min_temperature; // = 0.0;

    match sensor_subtype {
        SensorSubType::FridgeTag => { // duplicate cumulative breach config as a consecutive breach as well
            if let Some(temperature) = parse_float(&json_str["0"]["T AL"]) { // COLD
                max_temperature = max_breach_temperature;
                min_temperature = temperature;

                if let Some(duration) = parse_duration(&json_str["0"]["t AL"]) {
                    breach_configs.push(TemperatureBreachConfig {
                        breach_type: BreachType::ColdConsecutive,
                        maximum_temperature: max_temperature,
                        minimum_temperature: min_temperature,
                        duration: duration,
                    });
                    breach_configs.push(TemperatureBreachConfig {
                        breach_type: BreachType::ColdCumulative,
                        maximum_temperature: max_temperature,
                        minimum_temperature: min_temperature,
                        duration: duration,
                    });
                }
            }

            if let Some(temperature) = parse_float(&json_str["1"]["T AL"]) { // HOT
                min_temperature = min_breach_temperature;
                max_temperature = temperature;

                if let Some(duration) = parse_duration(&json_str["1"]["t AL"]) {
                    breach_configs.push(TemperatureBreachConfig {
                        breach_type: BreachType::HotConsecutive,
                        maximum_temperature: max_temperature,
                        minimum_temperature: min_temperature,
                        duration: duration,
                    });
                    breach_configs.push(TemperatureBreachConfig {
                        breach_type: BreachType::HotCumulative,
                        maximum_temperature: max_temperature,
                        minimum_temperature: min_temperature,
                        duration: duration,
                    });
                }
            }
        }
        SensorSubType::QTag => { // loop over fixed 5 alarms, but not all populated
            for config_index in 1..=5 {
                let json_config = &json_str[config_index.to_string()];

                if json_config.is_null() { // skip blank alarm
                    continue;
                } else {
                    if let Some(temperature) = parse_float(&json_config["T AL"]) { // breach temperature
                        if let Some(duration) = parse_duration(&json_config["t AL"]) { // breach duration threshold
                            if let Some(breach_type) = parse_int(&json_config["Type"]) { // breach type
                                match breach_type {
                                    1 => { // COLD_CONSECUTIVE
                                        max_temperature = max_breach_temperature;
                                        min_temperature = temperature;
                                    }
                                    2 => { // HOT_CONSECUTIVE
                                        min_temperature = min_breach_temperature;
                                        max_temperature = temperature;
                                    }
                                    3 => { // COLD_CUMULATIVE
                                        max_temperature = max_breach_temperature;
                                        min_temperature = temperature;
                                    }
                                    4 => { // HOT_CUMULATIVE
                                        min_temperature = min_breach_temperature;
                                        max_temperature = temperature;
                                    }
                                    _ => {
                                        // should never actually be used
                                        min_temperature = min_breach_temperature;
                                        max_temperature = max_breach_temperature;
                                    }
                                }

                                if let Some(temperature_breach_type) = qtag_breach_type(breach_type)
                                {
                                    breach_configs.push(TemperatureBreachConfig {
                                        breach_type: temperature_breach_type,
                                        maximum_temperature: max_temperature,
                                        minimum_temperature: min_temperature,
                                        duration: duration,
                                    })
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    if breach_configs.len() > 0 {
        Some(breach_configs)
    } else {
        None
    }
}

fn parse_fridgetag_breach(
    json_breach: &Value,
    json_config: &Value,
    breach_date: NaiveDate,
    breach_type: BreachType,
) -> Option<TemperatureBreach> {
    let mut breach_duration = Duration::seconds(0);
    let mut config_duration = Duration::seconds(0);
    let zero_time = NaiveTime::parse_from_str("00:00", "%H:%M").unwrap(); // hard-coded -> should always work!
    let mut start_time = zero_time;
    let mut valid_breach; // = true;

    if let Some(duration) = parse_duration(&json_breach["t Acc"]) { // breach duration > 0 for actual breach
        breach_duration = duration;
        valid_breach = breach_duration > Duration::seconds(0);
    } else {
        valid_breach = false;
    }

    if valid_breach {
        if let Some(duration) = parse_duration(&json_config["t AL"]) { // breach duration threshold
            config_duration = duration;
            valid_breach = config_duration > Duration::seconds(0);
        } else {
            valid_breach = false;
        }
        // Subtract breach duration from activation time to get start time 
        if let Some(breach_time) = parse_time(&json_breach["TS A"]) { // breach activation time
            if breach_time > zero_time + config_duration { // need to add zero_time to duration to make it a NaiveTime
                start_time = breach_time - config_duration
            } else {
                start_time = zero_time;
            }
        } else {
            valid_breach = false;
        }
    }

    if valid_breach {
        let breach_start_timestamp = NaiveDateTime::new(breach_date, start_time);

        let temperature_breach = TemperatureBreach {
            breach_type: breach_type,
            start_timestamp: breach_start_timestamp,
            end_timestamp: breach_start_timestamp + breach_duration, // only true for consecutive breaches, but this is all the data we have for FridgeTags
            duration: breach_duration,
            acknowledged: false,
        };
        Some(temperature_breach)
    } else {
        None
    }
}

fn qtag_breach_type(alarm_type: i64) -> Option<BreachType> {
    match alarm_type {
        1 => Some(BreachType::ColdConsecutive),
        2 => Some(BreachType::HotConsecutive),
        3 => Some(BreachType::ColdCumulative),
        4 => Some(BreachType::HotCumulative),
        _ => None,
    }
}

fn parse_qtag_breach(json_breach: &Value, alarm_type: i64) -> Option<Vec<TemperatureBreach>> {
    let mut temperature_breaches: Vec<TemperatureBreach> = Vec::new();
    let mut breach_index; // = 1;
    breach_index = 0; // for some weird reason, it didn't work if I just initialised it to zero above??

    loop { // loop as long as breach duration is valid
        if json_breach["t A"][breach_index].is_null() {
            break;
        } else {
            if let Some(breach_duration) = parse_duration(&json_breach["t A"][breach_index]) { // breach duration
                if let Some(breach_start_timestamp) =
                    parse_timestamp(&json_breach["TS S"][breach_index]) // breach start timestamp
                {
                    let mut breach_end_timestamp = breach_start_timestamp + breach_duration; // breach end timestamp is optional - default to start + duration
                    if let Some(breach_end) = parse_timestamp(&json_breach["TS E"][breach_index]) {
                        breach_end_timestamp = breach_end;
                    }

                    if let Some(breach_type) = qtag_breach_type(alarm_type) { // lookup breach type
                        temperature_breaches.push(TemperatureBreach {
                            breach_type: breach_type,
                            start_timestamp: breach_start_timestamp,
                            end_timestamp: breach_end_timestamp,
                            duration: breach_duration,
                            acknowledged: false,
                        });
                    }
                }
            }
        }

        breach_index = breach_index + 1;
    }

    if temperature_breaches.len() > 0 {
        Some(temperature_breaches)
    } else {
        None
    }
}

fn parse_breaches(
    json_str: &Value,
    sensor_subtype: &SensorSubType,
) -> Option<Vec<TemperatureBreach>> {
    let mut breaches: Vec<TemperatureBreach> = Vec::new();
    let mut alarm_index = 1;

    match sensor_subtype {
        SensorSubType::FridgeTag => loop { // loop until no more Hist elements
            let json_alarm = &json_str["Hist"][alarm_index.to_string()];

            if json_alarm.is_null() {
                break;
            } else {
                if let Some(breach_date) = parse_date(&json_alarm["Date"]) { // breach date
                    if let Some(temperature_breach) = parse_fridgetag_breach( // COLD_CUMULATIVE
                        &json_alarm["Alarm"]["0"],
                        &json_str["Conf"]["Alarm"]["0"],
                        breach_date,
                        BreachType::ColdCumulative,
                    ) {
                        breaches.push(temperature_breach);
                    }

                    if let Some(temperature_breach) = parse_fridgetag_breach( // HOT_CUMULATIVE
                        &json_alarm["Alarm"]["1"],
                        &json_str["Conf"]["Alarm"]["1"],
                        breach_date,
                        BreachType::HotCumulative,
                    ) {
                        breaches.push(temperature_breach);
                    }
                }
            }
            alarm_index = alarm_index + 1;
        },
        SensorSubType::QTag => { // 5 fixed alarms, not all populated
            for alarm_index in 1..=5 {
                let json_alarm = &json_str["Res"]["Alarm"][alarm_index.to_string()];

                if json_alarm.is_null() { // blank alarm -> proceed to next one
                    continue;
                } else {
                    if let Some(alarm_type) =
                        parse_int(&json_str["Conf"]["Alarm"][alarm_index.to_string()]["Type"]) // breach type
                    {
                        if let Some(temperature_breaches) =
                            parse_qtag_breach(&json_alarm, alarm_type) // can be multiple breaches
                        {
                            for breach_index in 0..=temperature_breaches.len() - 1 {
                                breaches.push(temperature_breaches[breach_index].clone());
                            }
                        }
                    }
                }
            }
        }
    }

    if breaches.len() > 0 {
        breaches.sort_unstable_by_key(|breaches| (breaches.start_timestamp,breaches.end_timestamp));
        Some(breaches)
    } else {
        None
    }
}

fn parse_logs(json_str: &Value, sensor_subtype: &SensorSubType) -> Option<Vec<TemperatureLog>> {
    let mut logs: Vec<TemperatureLog> = Vec::new();
    let mut log_index = 1;

    // First get the max/min temperature logs which are part of the alarm data

    match sensor_subtype {
        SensorSubType::FridgeTag => loop {
            let json_log = &json_str["Hist"][log_index.to_string()];

            if json_log.is_null() {
                break;
            } else {
                if let Some(log_date) = parse_date(&json_log["Date"]) {
                    if let Some(temperature_max) = parse_float(&json_log["Max T"]) {
                        if let Some(temperature_max_time) = parse_time(&json_log["TS Max T"]) {
                            let temperature_max_log = TemperatureLog {
                                timestamp: NaiveDateTime::new(log_date, temperature_max_time),
                                temperature: temperature_max,
                            };
                            logs.push(temperature_max_log);
                        }
                    }
                    if let Some(temperature_min) = parse_float(&json_log["Min T"]) {
                        if let Some(temperature_min_time) = parse_time(&json_log["TS Min T"]) {
                            let temperature_min_log = TemperatureLog {
                                timestamp: NaiveDateTime::new(log_date, temperature_min_time),
                                temperature: temperature_min,
                            };
                            logs.push(temperature_min_log);
                        }
                    }
                }

                log_index = log_index + 1;
            }
        },
        SensorSubType::QTag => {
            if let Some(temperature_min) = parse_float(&json_str["Res"]["Min T"]) { // min temperature
                if let Some(timestamp_min) = parse_timestamp(&json_str["Res"]["TS Min T"]) { // min timestamp
                    logs.push(TemperatureLog {
                        timestamp: timestamp_min,
                        temperature: temperature_min,
                    })
                }
            }
            if let Some(temperature_max) = parse_float(&json_str["Res"]["Max T"]) { // max temperature
                if let Some(timestamp_max) = parse_timestamp(&json_str["Res"]["TS Max T"]) { // max timestamp
                    logs.push(TemperatureLog {
                        timestamp: timestamp_max,
                        temperature: temperature_max,
                    })
                }
            }

            for alarm_index in 1..=5 { // loop over 5 fixed alarms, not all populated
                let json_alarm = &json_str["Res"]["Alarm"][alarm_index.to_string()];

                if json_alarm.is_null() { // skip blank alarm
                    continue;
                } else {
                    log_index = 0;

                    loop { // can have multiple entries for the same alarm - loop while valid
                        if json_alarm["T M"][log_index].is_null() {
                            break;
                        } else {
                            if let Some(log_temperature) =
                                parse_float(&json_alarm["T M"][log_index]) // alarm temperature
                            {
                                if let Some(log_timestamp) =
                                    parse_timestamp(&json_alarm["TS M"][log_index]) // alarm timestamp
                                {
                                    logs.push(TemperatureLog {
                                        timestamp: log_timestamp,
                                        temperature: log_temperature,
                                    })
                                }
                            }
                        }
                        log_index = log_index + 1;
                    }
                }
            }
        }
    }

    // Now process the raw temperature logs (if any)

    log_index = 0;

    loop { // loop over logs in Data section until no longer valid
        let json_log = &json_str["Data"];

        if json_log["Temperature"][log_index].is_null() {
            break;
        } else {
            if let Some(log_timestamp) = parse_timestamp(&json_log["Timestamp"][log_index]) { // timestamp
                if let Some(log_temperature) = parse_float(&json_log["Temperature"][log_index]) { // temperature
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

/// Reads sensor data from the specified sensor txt file.
pub fn read_sensor_from_file(file_path: &str) -> Option<Sensor> {

    if Path::new(file_path).exists() {

        let file_as_json = read_sensor_to_json(file_path);
        //println!("JSON: {:?}", file_as_json.to_string().replace("\"", ""));

        let report_timestamp: Option<NaiveDateTime>; // = None;
        let sensor_subtype = parse_subtype(&file_as_json);

        match sensor_subtype { // last timestamp in different places depending on sensor type
            SensorSubType::FridgeTag => {
                report_timestamp = parse_timestamp(&file_as_json["Hist"]["TS Report Creation"]);
            }
            SensorSubType::QTag => {
                report_timestamp = parse_timestamp(&file_as_json["Res"]["TS Stop"]);
            }
        }

        let sensor = Sensor {
            sensor_type: SensorType::Berlinger,
            serial: parse_string(&file_as_json["Conf"]["Serial"]),
            name: parse_string(&file_as_json["Device"]),
            last_connected_timestamp: report_timestamp,
            log_interval: parse_duration(&file_as_json["Conf"]["Logging Interval"]),
            breaches: parse_breaches(&file_as_json, &sensor_subtype),
            configs: parse_breach_configs(&file_as_json["Conf"]["Alarm"], &sensor_subtype),
            logs: parse_logs(&file_as_json, &sensor_subtype),
        };

        // Generate output file for debugging/reference
        let output_path = "sensor_".to_owned() + &sensor.serial + "_output.txt";
        if let Some(mut output) = File::create(&output_path).ok() {   
            if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
                println!("Output: {}", &output_path)
            }
        }

        Some(sensor)
    } else {
        println!("File not found: {}",file_path);
        None
    }
}

#[cfg(target_os = "macos")]
fn sensor_volume_paths() -> Vec<String> {

    let mut volume_list:Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir("/Volumes".to_str()) { // loop over folders in Volumes
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.is_dir() {
                    volume_list.push(entry.path().to_string());
                }
            }
        }
    }
    volume_list
}

#[cfg(target_os = "android")]
fn sensor_volume_paths() -> Vec<String> {

    let mut volume_list:Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir("/mnt/media_rw".to_str()) { // loop over mounted media folders
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.is_dir() {
                    volume_list.push(entry.path().to_string());
                }
            }
        }
    }
    volume_list
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn sensor_volume_paths() -> Vec<String> {

    let mut volume_list:Vec<String> = Vec::new();

    match drive_list() {
        Err(err) => println!("No drives found: {}",err),
        Ok(drives) => {
            for drive_index in 0..drives.len() { // loop over all detected drives

                let mount_points = &drives[drive_index].mountpoints;
                for partition_index in 0..mount_points.len() { // loop over partitions
                    let mount_point = &mount_points[partition_index];

                    if mount_point.totalBytes < Some(8*1024*1024*1024) { // possible USB drive if < 8 GB
                        volume_list.push(mount_point.path.clone());
                    }
                }
            }
        }
    }

    volume_list
}

fn sensor_file_list() -> Vec<String> {

    let mut file_list:Vec<String> = Vec::new();
    //let volume_paths = sensor_volume_paths();

    for volume_root in sensor_volume_paths() { // loop pver volumes

        if let Ok(entries) = fs::read_dir(&volume_root) { // loop over files in the volume root
            for entry in entries {
                if let Ok(entry) = entry {

                    if let Some(extension) = entry.path().extension() {
                        if extension == "txt" { // might be a sensor txt file
                            if let Some(txt_file_path) = entry.path().to_str() {
                                let pdf_file_path = txt_file_path.replace(".txt",".pdf");

                                if Path::new(&pdf_file_path).exists() { // but only if it has a matching PDF
                                    file_list.push(txt_file_path.to_string())
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    file_list
}

fn sensor_serial_from_file_path(txt_file_path: &str) -> Option<String> {

    let mut valid_serial = false;
    let mut serial = "";
    let file_path = Path::new(&txt_file_path);

    if file_path.exists() {
        if let Some(os_file_name) = file_path.file_name() {
            if let Some(file_name) = os_file_name.to_str() {
                let elements: Vec<&str> = file_name.split("_").collect();
                serial = elements[0];
                valid_serial = true;
            }
        }
    }

    if valid_serial {
        Some(serial.to_string())
    } else {
        None
    }
}

/// Returns all the serials found from currently mounted USB drives up to 8GB capacity
/// (-> any USB drive containing sensor files if you don't have a physical sensor).
/// For Berlinger sensors, it expects to find a serial_xxxxx.txt file in the root folder
/// together with a matching PDF file (USB drives can have multiple pairs of files).
pub fn read_sensor_serials() -> Option<Vec<String>> {

    let mut serial_list:Vec<String> = Vec::new();

    for txt_file_path in sensor_file_list() {
        
        if let Some(serial) = sensor_serial_from_file_path(&txt_file_path) {
            serial_list.push(serial)
        }
    }

    if serial_list.len() > 0 {
        Some(serial_list)
    }
    else {
        None
    }
}

/// Returns all sensors found from currently mounted USB drives up to 8GB capacity
/// (-> any USB drive containing sensor files if you don't have a physical sensor).
/// For Berlinger sensors, it expects to find a serial_xxxxx.txt file in the root folder
/// together with a matching PDF file (USB drives can have multiple pairs of files).
/// 
/// Currently using rs_drivelist -> only works for Windows and Linux so far...
pub fn read_sensors_from_usb() -> Option<Vec<Sensor>> {

    let mut sensors:Vec<Sensor> = Vec::new();

    for txt_file_path in sensor_file_list() {
        if let Some(sensor) = read_sensor_from_file(&txt_file_path) {
            sensors.push(sensor.clone())
        }
    }

    if sensors.len() > 0 {
        Some(sensors)
    } else {
        None
    }
}

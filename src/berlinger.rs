use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use serde_json::{json, Value};
use chrono::{NaiveDateTime,NaiveDate,NaiveTime,Duration};
use temperature_sensor::{Sensor,SensorType,BreachType,TemperatureLog,TemperatureBreach,TemperatureBreachConfig};

#[derive(Debug)]
enum SensorSubType {
    FridgeTag,
    QTag,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where P: AsRef<Path>, {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn count_whitespace_at_start(input: &str) -> usize {
    input
        .chars()
        .take_while(|ch| ch.is_whitespace() && *ch != '\n')
        .count()
}

fn read_sensor_to_json (file_path: &str) -> Value {

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
    let mut json_tag = String::new();
    let mut json_value = "";

    if let Ok(lines) = read_lines(file_path) {

        for line in lines {

            if let Ok(contents) = line {
                
                let level = count_whitespace_at_start(&contents);
                let elements = contents.split(", ");

                for element in elements {
                    
                    let json_elements: Vec<&str> = element.trim().split(": ").collect();
                    let last_char = json_elements[0].chars().last().unwrap();
                    let mut new_level = false;
                    let element_count = json_elements.len();

                    json_tag = json_elements[0].to_string();

                    if last_char == ':' {

                        json_tag.pop(); // remove trailing :
                        
                        if element_count == 1 { // new level if only one element in the line
                            new_level = true;
                        }
                    }

                    if new_level { // start of new level

                        match level {
                            0 => {
                                level_1 = json_tag.clone();
                                if level_1 != "Data" && level_1 != "Marker" {
                                    current_json[level_1.clone()] = json!({});
                                }
                            }
                            1 => {
                                level_2 = json_tag.clone();
                                current_json[level_1.clone()][level_2.clone()] = json!({});
                            }
                            2 => {
                                if level_1 == "Res" && level_2 == "Alarm" { // QTag can have multiple alarms for the same breach type
                                    breach_start_timestamps = Vec::new();
                                    breach_end_timestamps = Vec::new();
                                    breach_durations = Vec::new();
                                    breach_temperatures = Vec::new();
                                    breach_timestamps = Vec::new();
                                    breach_activation_timestamps = Vec::new();
                                }
                                level_3 = json_tag.clone();
                                current_json[level_1.clone()][level_2.clone()][level_3.clone()] = json!({});

                            }
                            3 => {
                                level_4 = json_tag.clone();
                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][level_4.clone()] = json!({});
                            }
                            _ => todo!()
                        }

                    } else {

                        if element_count > 1 { // regular line format

                            json_value = json_elements[1];
                            //println!("Value: {}, Tag: {}", json_value.to_string(), json_tag);

                            match level {
                                0 => {current_json[json_tag] = json_value.into()}
                                1 => {current_json[level_1.clone()][json_tag] = json_value.into()}
                                2 => {current_json[level_1.clone()][level_2.clone()][json_tag] = json_value.into()}
                                3 => {
                                    if level_1 == "Res" && level_2 == "Alarm" { // QTag breach
                                        match json_tag.as_str() {
                                            "TS S" => {
                                                breach_start_timestamps.push(json_value.into());
                                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = Value::Array(breach_start_timestamps.clone());
                                            }
                                            "TS E" => {
                                                breach_end_timestamps.push(json_value.into());
                                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = Value::Array(breach_end_timestamps.clone());
                                            }
                                            "t A" => {
                                                breach_durations.push(json_value.into());
                                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = Value::Array(breach_durations.clone());
                                            }
                                            "T M" => {
                                                breach_temperatures.push(json_value.into());
                                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = Value::Array(breach_temperatures.clone());
                                            }
                                            "TS M" => {
                                                breach_timestamps.push(json_value.into());
                                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = Value::Array(breach_timestamps.clone());
                                            }
                                            "TS A" => {
                                                breach_activation_timestamps.push(json_value.into());
                                                current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = Value::Array(breach_activation_timestamps.clone());
                                            }
                                            _=> todo!()
                                        }
                                    } else {
                                        current_json[level_1.clone()][level_2.clone()][level_3.clone()][json_tag] = json_value.into()
                                    }    
                                }
                                4 => {
                                        current_json[level_1.clone()][level_2.clone()][level_3.clone()][level_4.clone()][json_tag] = json_value.into()
                                }
                                _=> todo!()
                            }
                        } else { // tab-delimited line format

                            let tab_elements: Vec<&str> = json_tag.split("\t").collect();
                            if level_1 == "Data" {
                                data_timestamps.push(tab_elements[0].into());
                                data_temperatures.push(tab_elements[1].into());

                                if tab_elements.len() > 2 {
                                    data_breaches.push(Value::Bool(true));
                                } else {
                                    data_breaches.push(Value::Bool(false));
                                }
                                
                            }
                            if level_1 == "Marker" {
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
    json_str.to_string().replace("\"","")
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

fn parse_duration(json_str: &Value) -> Option<Duration> { // in minutes
    
    if let Some(minutes) = parse_int(json_str) {
        Some(Duration::minutes(minutes))
    } else {
        None
    }
}

fn parse_subtype(json_str: &Value) -> SensorSubType {
    if json_str["Hist"].is_null() {
        SensorSubType::QTag
    } else {
        SensorSubType::FridgeTag
    }
}

fn parse_breach_configs(json_str: &Value, sensor_subtype: &SensorSubType) -> Option<Vec<TemperatureBreachConfig>> {
    
    let mut breach_configs: Vec<TemperatureBreachConfig> = Vec::new();
    let max_breach_temperature = 100.0;
    let min_breach_temperature = -273.0;
    let mut max_temperature = 0.0;
    let mut min_temperature = 0.0;
    
    match sensor_subtype {

        SensorSubType::FridgeTag => {
            
            if let Some(temperature) = parse_float(&json_str["0"]["T AL"]) {

                max_temperature = temperature;
                min_temperature = min_breach_temperature;

                if let Some(duration) = parse_duration(&json_str["0"]["t AL"]) {
                    breach_configs.push(
                        TemperatureBreachConfig{
                            breach_type: BreachType::ColdConsecutive,
                            maximum_temperature: max_temperature, 
                            minimum_temperature: min_temperature, 
                            duration: duration, 
                        }
                    );
                    breach_configs.push(
                        TemperatureBreachConfig{
                            breach_type: BreachType::ColdCumulative,
                            maximum_temperature: max_temperature, 
                            minimum_temperature: min_temperature, 
                            duration: duration, 
                        }
                    );
                }
            }

            if let Some(temperature) = parse_float(&json_str["1"]["T AL"]) {

                min_temperature = temperature;
                max_temperature = max_breach_temperature;

                if let Some(duration) = parse_duration(&json_str["1"]["t AL"]) {
                    breach_configs.push(
                        TemperatureBreachConfig{
                            breach_type: BreachType::HotConsecutive,
                            maximum_temperature: max_temperature, 
                            minimum_temperature: min_temperature, 
                            duration: duration, 
                        }
                    );
                    breach_configs.push(
                        TemperatureBreachConfig{
                            breach_type: BreachType::HotCumulative,
                            maximum_temperature: max_temperature, 
                            minimum_temperature: min_temperature, 
                            duration: duration, 
                        }
                    );
                }
            }
        },
        SensorSubType::QTag => {

            for config_index in 1..=5 {

                let json_config = &json_str[config_index.to_string()];
 
                if json_config.is_null() {
                    continue;
                } else {
                    if let Some(temperature) = parse_float(&json_config["T AL"]) {
                        if let Some(duration) = parse_duration(&json_config["t AL"]) {
                            if let Some(breach_type) = parse_int(&json_config["Type"]) {

                                match breach_type {
                                    1 => {
                                        max_temperature = temperature;
                                        min_temperature = min_breach_temperature;
                                    }
                                    2 => {
                                        min_temperature = temperature;
                                        max_temperature = max_breach_temperature;
                                    }
                                    3 => {
                                        max_temperature = temperature;
                                        min_temperature = min_breach_temperature;
                                    }
                                    4 => {
                                        min_temperature = temperature;
                                        max_temperature = max_breach_temperature;
                                    }
                                    _=> { // never actually used
                                        min_temperature = min_breach_temperature;
                                        max_temperature = max_breach_temperature;
                                    }
                                }

                                if let Some(temperature_breach_type) = qtag_breach_type(breach_type) {
                                    breach_configs.push(
                                        TemperatureBreachConfig { 
                                            breach_type: temperature_breach_type, 
                                            maximum_temperature: max_temperature, 
                                            minimum_temperature: min_temperature, 
                                            duration: duration, 
                                        }
                                    )
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

fn parse_fridgetag_breach(json_breach: &Value, json_config: &Value, breach_date: NaiveDate) -> Option<TemperatureBreach> {

    let mut breach_duration = Duration::seconds(0);
    let mut config_duration = Duration::seconds(0);
    let zero_time = NaiveTime::parse_from_str("00:00", "%H:%M").unwrap();
    let mut start_time = zero_time;
    let mut valid_breach = true;

    if let Some(duration) = parse_duration(&json_breach["t Acc"]) {
        breach_duration = duration;
        valid_breach = breach_duration > Duration::seconds(0);
    } else {
        valid_breach = false;
    }
    
    if valid_breach {

        if let Some(duration) = parse_duration(&json_config["t AL"]) {
            config_duration = duration;
            valid_breach = config_duration > Duration::seconds(0);
        } else {
            valid_breach = false;
        }
        if let Some(breach_time) = parse_time(&json_breach["TS A"]) {

            if breach_time > zero_time + config_duration {
                start_time = breach_time - config_duration
            } else {
                start_time = zero_time;
            }

        } else {
            valid_breach = false;
        }
    }
    
    if valid_breach {

        let breach_start_timestamp = NaiveDateTime::new(breach_date,start_time);

        let temperature_breach = TemperatureBreach{
            breach_type: BreachType::HotCumulative, // default - overridden in calling function
            start_timestamp: breach_start_timestamp,
            end_timestamp: breach_start_timestamp + breach_duration,
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
        _=> None,
    }
}

fn parse_qtag_breach(json_breach: &Value, alarm_type: i64) -> Option<Vec<TemperatureBreach>> {

    let mut temperature_breaches: Vec<TemperatureBreach> = Vec::new();
    let mut breach_index = 1;
    breach_index = 0;

    loop {

        if json_breach["t A"][breach_index].is_null() {
            break;
        } else {

            if let Some(breach_duration) = parse_duration(&json_breach["t A"][breach_index]) {

                if let Some(breach_start_timestamp) = parse_timestamp(&json_breach["TS S"][breach_index]) {

                    let mut breach_end_timestamp = breach_start_timestamp+breach_duration;
                    if let Some(breach_end) = parse_timestamp(&json_breach["TS E"][breach_index]) {
                        breach_end_timestamp = breach_end;
                    }

                    if let Some(breach_type) = qtag_breach_type(alarm_type) {
                        temperature_breaches.push(
                            TemperatureBreach {
                                breach_type: breach_type,
                                start_timestamp: breach_start_timestamp,
                                end_timestamp: breach_end_timestamp,
                                duration: breach_duration,
                                acknowledged: false, 
                            }
                        );
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

fn parse_breaches(json_str: &Value, sensor_subtype: &SensorSubType) -> Option<Vec<TemperatureBreach>> {
    
    let mut breaches: Vec<TemperatureBreach> = Vec::new();
    let mut alarm_index = 1;

    match sensor_subtype {
        SensorSubType::FridgeTag => {

            loop {
                let json_alarm = &json_str["Hist"][alarm_index.to_string()];
                
                if json_alarm.is_null() {
                    break;
                } else {
                    if let Some(breach_date) = parse_date(&json_alarm["Date"]) {
                        
                        if let Some(mut temperature_breach) = parse_fridgetag_breach(&json_alarm["Alarm"]["0"], &json_str["Conf"]["Alarm"]["0"], breach_date) {
                            temperature_breach.breach_type = BreachType::ColdCumulative;
                            breaches.push(temperature_breach);
                        }

                        if let Some(mut temperature_breach) = parse_fridgetag_breach(&json_alarm["Alarm"]["1"], &json_str["Conf"]["Alarm"]["1"], breach_date) {
                            temperature_breach.breach_type = BreachType::HotCumulative;
                            breaches.push(temperature_breach);
                        }
                    }
                }
                alarm_index = alarm_index + 1;
            }
        },
        SensorSubType::QTag => {

            for alarm_index in 1..=5 {

                let json_alarm = &json_str["Res"]["Alarm"][alarm_index.to_string()];
                
                if json_alarm.is_null() {
                    continue;
                } else {
                    if let Some(alarm_type) = parse_int(&json_str["Conf"]["Alarm"][alarm_index.to_string()]["Type"]) {
                        
                        if let Some(temperature_breaches) = parse_qtag_breach(&json_alarm, alarm_type) {
                            for breach_index in 0..=temperature_breaches.len()-1 {
                                breaches.push(temperature_breaches[breach_index].clone());
                            }
                        }
                    }
                }
            }
        }
    }

    if breaches.len() > 0 {
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
        SensorSubType::FridgeTag => {

            loop {

                let json_log = &json_str["Hist"][log_index.to_string()];
        
                if json_log.is_null() {
                    break;
                } else {
                           
                    if let Some(log_date) = parse_date(&json_log["Date"]) {
                                
                        if let Some(temperature_max) = parse_float(&json_log["Max T"]) {
                                
                            if let Some(temperature_max_time) = parse_time(&json_log["TS Max T"]) {
                                let temperature_max_log = TemperatureLog {
                                    timestamp: NaiveDateTime::new(log_date,temperature_max_time),
                                    temperature: temperature_max,
                                };
                                logs.push(temperature_max_log);
                            }
                        }
                        if let Some(temperature_min) = parse_float(&json_log["Min T"]) {
                              
                            if let Some(temperature_min_time) = parse_time(&json_log["TS Min T"]) {
                                let temperature_min_log = TemperatureLog {
                                    timestamp: NaiveDateTime::new(log_date,temperature_min_time),
                                    temperature: temperature_min,
                                };
                                logs.push(temperature_min_log);
                            }
                        }                       
                    }
                
                    log_index = log_index + 1;
                }
            }
        
        },
        SensorSubType::QTag => {

            if let Some(temperature_min) = parse_float(&json_str["Res"]["Min T"]) {
                if let Some(timestamp_min) = parse_timestamp(&json_str["Res"]["TS Min T"]) {
                    logs.push(TemperatureLog {
                        timestamp: timestamp_min,
                        temperature: temperature_min,
                    })
                }
            }
            if let Some(temperature_max) = parse_float(&json_str["Res"]["Max T"]) {
                if let Some(timestamp_max) = parse_timestamp(&json_str["Res"]["TS Max T"]) {
                    logs.push(TemperatureLog {
                        timestamp: timestamp_max,
                        temperature: temperature_max,
                    })
                }
            }

            for alarm_index in 1..=5 {

                let json_alarm = &json_str["Res"]["Alarm"][alarm_index.to_string()];
                
                if json_alarm.is_null() {
                    continue;
                } else {

                    log_index = 0;
                    
                    loop {       
                        if json_alarm["T M"][log_index].is_null() {
                            break;
                        } else {
                    
                            if let Some(log_temperature) = parse_float(&json_alarm["T M"][log_index]) {
                                if let Some(log_timestamp) = parse_timestamp(&json_alarm["TS M"][log_index]) {
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
  
    loop {

        let json_log = &json_str["Data"];

        if json_log["Temperature"][log_index].is_null() {
            break;
        } else {
       
            if let Some(log_timestamp) = parse_timestamp(&json_log["Timestamp"][log_index]) {          
                if let Some(log_temperature) = parse_float(&json_log["Temperature"][log_index]) {                    
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
        Some(logs)
    } else {
        None
    }

}

pub fn read_sensor_file(file_path: &str) -> Option<Sensor> {

    let file_as_json = read_sensor_to_json(file_path);

    println!("JSON: {:?}",file_as_json.to_string().replace("\"", ""));

    let mut report_timestamp: Option<NaiveDateTime> = None;
    let sensor_subtype = parse_subtype(&file_as_json);
    
    match sensor_subtype {
        SensorSubType::FridgeTag => {
            report_timestamp = parse_timestamp(&file_as_json["Hist"]["TS Report Creation"]);
        },
        SensorSubType::QTag => {
             report_timestamp = parse_timestamp(&file_as_json["Res"]["TS Stop"]);
        }
    }

    let sensor = Sensor {
        sensor_type: SensorType::Berlinger,
        registration: parse_string(&file_as_json["Conf"]["Serial"]),
        name: parse_string(&file_as_json["Device"]),
        last_connected_timestamp: report_timestamp,
        log_interval: parse_duration(&file_as_json["Conf"]["Logging Interval"]),
        breaches: parse_breaches(&file_as_json, &sensor_subtype),
        configs: parse_breach_configs(&file_as_json["Conf"]["Alarm"], &sensor_subtype),
        logs: parse_logs(&file_as_json, &sensor_subtype),
    };

    Some(sensor)
}
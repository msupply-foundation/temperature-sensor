//! # Temperature Sensor
//! 
//! `temperature_sensor` is a collection of utilities to parse data files
//! generated from temperature sensors and return details of the sensor,
//! its breach configurations, recorded breaches and temperature logs in
//! a standard format
//! 
//! It has been implemented for use in our open mSupply LMIS software, which 
//! is being rewritten in Rust (https://msupply.foundation/projects/omsupply)
//! 
//! So far it only supports Berlinger FridgeTag and QTag USB sensors
//! (https://www.berlinger.com/cold-chain-management) but it is hoped to extend
//! it to other sensor types in future

pub mod berlinger;
pub mod common;

use crate::common::{
    BreachType, Sensor, SensorType, TemperatureBreach, TemperatureBreachConfig, TemperatureLog,
};

use chrono::{Duration, NaiveDateTime};

/// Returns some made-up example temnperature sensor data
pub fn sample_sensor() -> Sensor {
    let config_cold_consecutive = TemperatureBreachConfig {
        breach_type: BreachType::ColdConsecutive,
        maximum_temperature: 100.0,
        minimum_temperature: 2.0,
        duration: Duration::seconds(240),
    };

    let config_hot_consecutive = TemperatureBreachConfig {
        breach_type: BreachType::HotConsecutive,
        maximum_temperature: 8.0,
        minimum_temperature: -273.0,
        duration: Duration::seconds(300),
    };

    let temperature_values = vec![
        3.5, 4.0, 5.0, 7.5, // ok
        8.8, 9.2, 8.7, 9.1, 8.4, 8.2, 8.1, //hot
        7.9, 3.2, // ok
        1.2, 1.3, 0.4, -0.2, 0.7, // cold
        2.5, // ok
    ];

    let mut temperature_timestamp =
        NaiveDateTime::parse_from_str("2023-05-23 13:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let interval = Duration::minutes(1);
    let hot_start_timestamp = temperature_timestamp + interval * 4;
    let hot_end_timestamp = temperature_timestamp + interval * 10;
    let hot_duration = hot_end_timestamp - hot_start_timestamp; //interval*(10-4);
    let cold_start_timestamp = temperature_timestamp + interval * 13;
    let cold_end_timestamp = temperature_timestamp + interval * 17;
    let cold_duration = cold_end_timestamp - cold_start_timestamp; //interval*(17-13);

    let temperature_iterator = temperature_values.iter();
    let mut temperature_logs: Vec<TemperatureLog> = Vec::new();

    for temperature_value in temperature_iterator {
        temperature_logs.push(TemperatureLog {
            temperature: *temperature_value,
            timestamp: temperature_timestamp,
        });
        temperature_timestamp = temperature_timestamp + interval;
    }

    let breach_cold_consecutive = TemperatureBreach {
        breach_type: BreachType::ColdConsecutive,
        start_timestamp: cold_start_timestamp,
        end_timestamp: cold_end_timestamp,
        duration: cold_duration,
        acknowledged: false,
    };

    let breach_hot_consecutive = TemperatureBreach {
        breach_type: BreachType::HotConsecutive,
        start_timestamp: hot_start_timestamp,
        end_timestamp: hot_end_timestamp,
        duration: hot_duration,
        acknowledged: false,
    };

    let sensor = Sensor {
        sensor_type: SensorType::Berlinger,
        registration: String::from("reg 1234"),
        name: String::from("Berlinger 1"),
        last_connected_timestamp: Some(temperature_timestamp),
        log_interval: Some(interval),
        breaches: Some(vec![breach_hot_consecutive, breach_cold_consecutive]),
        configs: Some(vec![config_cold_consecutive, config_hot_consecutive]),
        logs: Some(temperature_logs),
    };

    sensor
}

/// 
/// 
pub fn read_connected_sensors() -> Result<Vec<Sensor>, String> {
    
    if let Some(sensor_array) = berlinger::read_sensors_from_usb() {
        Ok(sensor_array)
    } else {
        Err("No sensors found".to_string())
    }
}

pub fn read_connected_serials() -> Result<Vec<String>, String> {

    if let Some(sensor_serials) = berlinger::read_sensor_serials() {
        println!("Serials found: {:?}",sensor_serials);
        Ok(sensor_serials)
    } else {
        Err("No sensors found".to_string())
    }
}

pub fn read_sensor_file(file_path: &str) -> Result<Sensor, String> {
    
    if let Some(sensor) = berlinger::read_sensor_from_file(&file_path) {
        Ok(sensor)
    } else {
        Err("Sensor file not found".to_string())
    }
}

//! # Temperature Sensor
//!
//! `temperature_sensor` is a collection of utilities to parse data files
//! generated from temperature sensors and return details of the sensor,
//! its breach configurations, recorded breaches and temperature logs in
//! a standard format.
//!
//! It has been implemented for use in our open mSupply LMIS software, which
//! is being rewritten in Rust <https://msupply.foundation/projects/omsupply>.
//!
//! So far it only supports Berlinger Fridge-tag and Q-tag USB sensors
//! <https://www.berlinger.com/cold-chain-management>, and LogTag USB sensors
//! <https://logtagrecorders.com> but it is hoped to extend it to other sensor
//! types in future.
//!
//! (1) Berlinger Fridge-tags without logging e.g. Fridge-tag 2 or Fridge-tag 2E:
//!
//! Temperature logs are only recorded for the max & min temperature each day. Alarms are
//! single-event i.e. triggered when the temperature is continuously out of range for longer
//! than the alarm duration, so they are returned as consecutive breaches, where:
//!     Alarm=0 => cold (consecutive) breach
//!     Alarm=1 => hot (consecutive) breach
//!
//! Each day also records the min, max & average temperature and the total (cumulative) time
//! above/below the alarm temperatures. That cumulative time never triggers an alarm, so it is
//! returned in the sensor's daily summaries rather than as a breach.
//!
//! Note that the sensor doesn't record the start or end of the breach, only the time
//! when the breach was triggered and the total time out of range that day
//! => we work out the breach start time by subtracting the breach config duration
//! from the breach trigger time (or midnight if that is later) and we work out
//! the breach end time by adding the total breach duration to the breach start time
//! (or midnight if that is earlier).
//!
//! Obviously, these calculations will only be correct if the breach is continuous i.e.
//! there are no gaps when the temperature is not breaching, but it's the best that can
//! be done with the limited data available.
//!
//! (2) Berlinger Fridge-tags with logging e.g. Fridge-tag 2L or Fridge-tag UL:
//!
//! These record breaches in the same way and have the same limitations, but they also
//! record full temperature logs (usually every 5 minutes) => it is possible to process
//! these logs to calculate the actual start & end of each breach, assuming that the same
//! breach configurations apply (i.e. the same temperature & duration thresholds). The sensor
//! only has consecutive breach configurations, so detecting cumulative breaches from the logs
//! needs cumulative configurations to be supplied. Unlike the alarms recorded by the sensor, which are
//! cut off at midnight, breaches calculated from logs have the potential to cover more than
//! one day if they are ongoing at midnight, and of course you can have multiple breaches
//! per day.
//!
//! (3) Berlinger Q-tags (all with logging?) e.g. Q-tag CLm doc LR:
//!
//! These can have up to 5 breach configurations, each having one of the following types:
//!     Alarm type=1 => cold consecutive breach
//!     Alarm type=2 => hot consecutive breach
//!     Alarm type=3 => cold cumulative breach
//!     Alarm type=4 => hot cumulative breach
//!
//! Q-tags record the start and end time (and duration) of consecutive breaches,
//! although they only record the start time of cumulative breaches, so the end time
//! still needs to be calculated in the same way as for Fridge-tags i.e. by adding the
//! breach duration to the start time. For non-continuous cumulative breaches, the
//! true end time can be calculated from the last breaching temperature log of the day.
//!
//! (4) LogTags (all with logging):
//!
//! These can have multiple breach configurations, but they are not stored in the USB tag's
//! CSV file - just the minimum and maximum temperatures for the allowed range. We can generate
//! default hot and cold breach configurations based on these temperatures and the logging
//! interval - we apply a default consecutive breach threshold of 10x the logging interval
//! for consecutive breaches and 20x for cumulative breaches, and then we can process the
//! temperature logs to calculate any breaches based on these default configurations.
//!
//! We have added a new `calculate_sensor_breaches` function which allows you to either
//! calculate breaches for any (optional) arbitrary breach configuration passed in, or for
//! all existing breach configurations of the sensor. While this was implemented primarily
//! for LogTags, the same function can be used to (re)calculate the reported breaches for
//! any Berlinger Fridge-tags which have logging data, and also to extend them by adding new
//! arbitrary breach configurations (e.g. use different min/max temperatures and/or breach
//! duration thresholds).

pub mod berlinger;
pub mod common;
pub mod logtag;

use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[cfg(any(target_os = "windows", target_os = "linux"))]
use rs_drivelist::drive_list;

pub use crate::common::{
    BreachType, DailySummary, Sensor, SensorType, TemperatureBreach, TemperatureBreachConfig,
    TemperatureLog,
};

use chrono::{Datelike, Duration, Local, NaiveDateTime};

/// Returns some made-up example temperature sensor data, for use in automated tests.
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
        serial: String::from("reg 1234"),
        name: String::from("Berlinger 1"),
        last_connected_timestamp: Some(temperature_timestamp),
        log_interval: Some(interval),
        breaches: Some(vec![breach_hot_consecutive, breach_cold_consecutive]),
        configs: Some(vec![config_cold_consecutive, config_hot_consecutive]),
        logs: Some(temperature_logs),
        daily_summaries: None,
    };

    sensor
}

fn sensor_type_from_filename(file_path: &str) -> SensorType {
    if file_path.contains("LogTag") {
        SensorType::LogTag
    } else {
        SensorType::Berlinger
    }
}

/// Returns all sensors found from currently mounted USB drives up to 8GB capacity
/// (=> any USB drive containing sensor files if you don't have a physical sensor).
///
/// For Berlinger sensors, it expects to find a serial_xxxxx.txt file in the root folder
/// together with a matching PDF file (USB drives can have multiple pairs of files).
///
/// For LogTag sensors, it expects to find a LogTag_serial_xxxxx.csv file in the root folder
///
pub fn read_connected_sensors() -> Result<Vec<Sensor>, String> {
    if let Some(sensor_array) = read_sensors_from_usb() {
        Ok(sensor_array)
    } else {
        Err("No sensors found".to_string())
    }
}

/// Returns all the serials found from currently mounted USB drives up to 8GB capacity
/// (=> any USB drive containing sensor files if you don't have a physical sensor).
///
/// For Berlinger sensors, it expects to find a serial_xxxxx.txt file in the root folder
/// together with a matching PDF file (USB drives can have multiple pairs of files).
///
/// For LogTag sensors, it expects to find a LogTag_serial_xxxxx.csv file in the root folder
///
pub fn read_connected_serials() -> Result<Vec<String>, String> {
    if let Some(sensor_serials) = read_sensor_serials() {
        log::info!("Serials found: {:?}", sensor_serials);
        Ok(sensor_serials)
    } else {
        Err("No sensors found".to_string())
    }
}

/// Reads sensor data from the specified sensor txt file.
pub fn read_sensor_file(file_path: &str) -> Result<Sensor, String> {
    match sensor_type_from_filename(file_path) {
        SensorType::Berlinger => {
            if let Some(sensor) = berlinger::read_sensor_from_file(&file_path) {
                if cfg!(debug_assertions) {
                    // Generate output file for debugging/reference
                    let output_path = "sensor_".to_owned() + &sensor.serial + "_output.txt";
                    if let Some(mut output) = File::create(&output_path).ok() {
                        if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
                            log::info!("Output: {}", &output_path)
                        }
                    }
                }
                Ok(sensor)
            } else {
                Err("Sensor file not found".to_string())
            }
        }
        SensorType::LogTag => {
            if let Some(mut sensor) = logtag::read_sensor_from_file(&file_path) {
                sensor.breaches = calculate_sensor_breaches(&sensor, None);
                if cfg!(debug_assertions) {
                    // Generate output file for debugging/reference
                    let output_path = "sensor_".to_owned() + &sensor.serial + "_output.txt";
                    if let Some(mut output) = File::create(&output_path).ok() {
                        if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
                            log::info!("Output: {}", &output_path)
                        }
                    }
                }
                Ok(sensor)
            } else {
                Err("Sensor file not found".to_string())
            }
        }
    }
}

/// Reads sensor data from the contents of a txt file, by writing the
/// contents to a local txt file and reading that.
pub fn parse_sensor(file_contents: &str) -> Result<Sensor, String> {
    let file_path = format!("sensor_input_{}.txt", Local::now().timestamp());
    if let Some(mut output) = File::create(&file_path).ok() {
        if write!(output, "{}", file_contents).is_ok() {
            log::info!("Reading sensor from: {}", &file_path);
            return read_sensor_file(&file_path);
        }
    }
    Err("Sensor file not created".to_string())
}

/// Reads sensor data from USB for the txt file corresponding to the specified serial.
/// Note that the serial is expected to match the corresponding serial field inside
/// the txt file.
pub fn read_sensor(serial: &str) -> Result<Sensor, String> {
    if let Some(sensor_array) = read_sensors_from_usb() {
        for sensor in sensor_array {
            if sensor.serial == serial.to_string() {
                log::info!("Found sensor: {}", serial);

                if cfg!(debug_assertions) {
                    // Generate output file for debugging/reference
                    let output_path = "sensor_".to_owned() + &sensor.serial + "_output.txt";
                    if let Some(mut output) = File::create(&output_path).ok() {
                        if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
                            log::info!("Output: {}", &output_path)
                        }
                    }
                }

                return Ok(sensor);
            }
        }
    }

    return Err("Sensor not found".to_string());
}

fn create_breach(
    breach_type: &BreachType,
    breach_start: NaiveDateTime,
    breach_end: NaiveDateTime,
    breach_duration: Duration,
    config_duration: Duration,
) -> Option<TemperatureBreach> {
    if breach_duration > config_duration {
        let temperature_breach = TemperatureBreach {
            breach_type: breach_type.clone(),
            start_timestamp: breach_start,
            end_timestamp: breach_end,
            duration: breach_duration,
            acknowledged: false,
        };
        Some(temperature_breach)
    } else {
        None
    }
}

/// Scans through the sensor's temperature logs and works out breaches, based on either
/// the passed in TemperatureBreachConfig or the sensor's existing breach configs.
///
/// Consecutive breaches are returned if there are consecutive out-of-range temperature
/// logs spanning at least the specified duration, and these can span multiple days.
///
/// Cumulative breaches are returned if there are enough out-of-range temperature logs
/// within each 24-hour day in one or more separate breach episodes to accumulate at least
/// the specified duration. If they span midnight, they will be split into separate breaches.
///
/// Note that the difference between the start and end breach timestamps is only
/// the same as the breach duration for consecutive breaches. The breach duration for a
/// cumulative breach is the sum of the individual breach episodes.
pub fn calculate_sensor_breaches(
    sensor: &Sensor,
    breach_config: Option<TemperatureBreachConfig>,
) -> Option<Vec<TemperatureBreach>> {
    let mut breaches: Vec<TemperatureBreach> = Vec::new();
    let mut configs: Vec<TemperatureBreachConfig> = Vec::new();

    if let Some(sensor_config) = breach_config {
        configs.push(sensor_config); // use passed in config if it exists
    } else {
        match &sensor.configs {
            Some(sensor_configs) => {
                configs = sensor_configs.clone(); // otherwise use the sensor configs
            }
            None => {}
        };
    }

    for config in configs {
        match &sensor.logs {
            Some(logs) => {
                let mut breached = false;
                let mut breaching: bool;
                let mut total_breach_duration = Duration::seconds(0);
                let consecutive_breach = config.breach_type == BreachType::ColdConsecutive
                    || config.breach_type == BreachType::HotConsecutive;
                let mut start_timestamp = logs[0].timestamp;
                let last_timestamp = logs[logs.len() - 1].timestamp;
                let mut breach_end_timestamp = last_timestamp;
                let mut breach_start_timestamp = start_timestamp;

                for log in logs {
                    match config.breach_type {
                        BreachType::HotConsecutive => {
                            breaching = log.temperature > config.maximum_temperature;
                        }
                        BreachType::ColdConsecutive => {
                            breaching = log.temperature < config.minimum_temperature;
                        }
                        BreachType::HotCumulative => {
                            breaching = log.temperature > config.maximum_temperature;
                        }
                        BreachType::ColdCumulative => {
                            breaching = log.temperature < config.minimum_temperature;
                        }
                    };

                    if breached {
                        // If it's an ongoing breach:
                        // - if we've got to the end of the temperature logs => then end it.
                        // - if it's a consecutive breach and we're no longer breaching => then end it.
                        // - if it's a cumulative one spanning midnight => then end it at midnight
                        //   and start a new one.
                        // - if it's a cumulative breach and we're no longer breaching => then end the
                        //   current episode and update the breach duration.

                        if log.timestamp == last_timestamp {
                            // end breach if we're at the last log
                            breach_end_timestamp = log.timestamp;
                            total_breach_duration =
                                total_breach_duration + (breach_end_timestamp - start_timestamp); // could be cumulative
                            if let Some(temperature_breach) = create_breach(
                                &config.breach_type,
                                breach_start_timestamp,
                                breach_end_timestamp,
                                total_breach_duration,
                                config.duration,
                            ) {
                                breaches.push(temperature_breach);
                            }
                        } else {
                            if consecutive_breach {
                                if breaching { // nothing to do
                                } else {
                                    // no longer breaching -> end current breach and reset duration
                                    breached = false;
                                    breach_end_timestamp = log.timestamp;
                                    total_breach_duration = breach_end_timestamp - start_timestamp;
                                    if let Some(temperature_breach) = create_breach(
                                        &config.breach_type,
                                        breach_start_timestamp,
                                        breach_end_timestamp,
                                        total_breach_duration,
                                        config.duration,
                                    ) {
                                        breaches.push(temperature_breach);
                                    }
                                    total_breach_duration = Duration::seconds(0);
                                }
                            } else {
                                // cumulative breach
                                if breaching {
                                    // end breach if it's a new day, and start a new one
                                    if start_timestamp.day() < log.timestamp.day() {
                                        breach_end_timestamp =
                                            log.timestamp.date().and_hms_opt(0, 0, 0).unwrap(); // set to midnight
                                        total_breach_duration = total_breach_duration
                                            + (breach_end_timestamp - start_timestamp);
                                        if let Some(temperature_breach) = create_breach(
                                            &config.breach_type,
                                            breach_start_timestamp,
                                            breach_end_timestamp,
                                            total_breach_duration,
                                            config.duration,
                                        ) {
                                            breaches.push(temperature_breach);
                                        }
                                        start_timestamp = breach_end_timestamp;
                                        breach_start_timestamp = breach_end_timestamp;
                                        total_breach_duration = Duration::seconds(0);
                                    }
                                } else {
                                    // no longer breaching => end this episode and update duration;
                                    // if this happens over midnight, then use that as the end time
                                    if start_timestamp.day() < log.timestamp.day() {
                                        breach_end_timestamp =
                                            log.timestamp.date().and_hms_opt(0, 0, 0).unwrap();
                                    } else {
                                        breach_end_timestamp = log.timestamp;
                                    }
                                    breached = false;
                                    total_breach_duration = total_breach_duration
                                        + (breach_end_timestamp - start_timestamp);
                                }
                            }
                        };
                    } else {
                        // If we're not in an ongoing breach:
                        // - if we've just started a new breach => set the episode start time and
                        //   set the breach start if it's a consecutive breach or the first episode
                        //   of a cumulative one.
                        // - if we're at the end => end any unfinished (cumulative) breach.

                        if breaching {
                            // new breach - store the start time
                            start_timestamp = log.timestamp;
                            breached = true;
                            if total_breach_duration == Duration::seconds(0) {
                                // only reset breach start if there hasn't already been a cumulative breach episode
                                breach_start_timestamp = start_timestamp;
                            }
                        } else {
                            if log.timestamp == last_timestamp {
                                // end any unsaved cumulative breach if we're at the last log
                                if let Some(temperature_breach) = create_breach(
                                    &config.breach_type,
                                    breach_start_timestamp,
                                    breach_end_timestamp,
                                    total_breach_duration,
                                    config.duration,
                                ) {
                                    breaches.push(temperature_breach);
                                }
                            }
                        }
                    }
                }
            }
            None => {}
        };
    }

    if cfg!(debug_assertions) {
        // Generate output file for debugging/reference
        let output_path = "sensor_".to_owned() + &sensor.serial + "_breach_output.txt";
        if let Some(mut output) = File::create(&output_path).ok() {
            if write!(output, "{}", format!("{:?}\n\n", breaches)).is_ok() {
                log::info!("Breach output: {}", &output_path);
            }
        }
    }

    if breaches.len() > 0 {
        Some(breaches)
    } else {
        None
    }
}

/// Applies optional start/end timestamps to the breaches, temperature logs and
/// daily summaries of the specified sensor e.g. to include only data since the last time the
/// sensor was read (or from the start of the last recorded breach if it was
/// ongoing at the time of the last sensor read).
///
/// Temperature logs are filtered out if they are either before the start timestamp
/// or after the end timestamp.
///
/// Breaches are filtered out if they are entirely before the start timestamp or after
/// the end timestamp i.e. keep if any part of the breach is between the start timestamp
/// and the end timestamp.
///
/// Daily summaries are filtered out if their date is more than a day before the date of the
/// start timestamp, or after the date of the end timestamp. The day being recorded when the
/// sensor was last read is kept, as its summary is likely to have been incomplete, and so is
/// the day before, as summary dates are the sensor's local dates while the start timestamp
/// may be in another time zone (e.g. UTC).
///
/// Note that the difference between the start and end breach timestamps is only
/// the same as the breach duration for consecutive breaches which start and end
/// within the specified interval.
pub fn filter_sensor(
    mut sensor: Sensor,
    start_timestamp: Option<NaiveDateTime>,
    end_timestamp: Option<NaiveDateTime>,
) -> Sensor {
    if let Some(start) = start_timestamp {
        let mut filtered_logs: Vec<TemperatureLog> = Vec::new();
        match sensor.logs {
            Some(logs) => {
                for log in logs {
                    if log.timestamp >= start {
                        filtered_logs.push(log);
                    }
                }
                if filtered_logs.len() > 0 {
                    sensor.logs = Some(filtered_logs);
                } else {
                    sensor.logs = None;
                }
            }
            None => {}
        };
        let mut filtered_breaches: Vec<TemperatureBreach> = Vec::new();
        match sensor.breaches {
            Some(breaches) => {
                for breach in breaches {
                    if breach.start_timestamp >= start {
                        // keep if start of breach is after start timestamp
                        filtered_breaches.push(breach);
                    } else if breach.end_timestamp >= start {
                        // if start of breach is before start timestamp
                        filtered_breaches.push(breach); // keep if end of breach is after start timestamp
                    }
                }
                if filtered_breaches.len() > 0 {
                    sensor.breaches = Some(filtered_breaches);
                } else {
                    sensor.breaches = None;
                }
            }
            None => {}
        };
        if let Some(daily_summaries) = sensor.daily_summaries {
            // keep the day being recorded at the start timestamp, as its summary may have been
            // incomplete (allowing a day for the start timestamp being in another time zone)
            let start_date = start.date() - Duration::days(1);
            let filtered_summaries: Vec<DailySummary> = daily_summaries
                .into_iter()
                .filter(|daily_summary| daily_summary.date >= start_date)
                .collect();
            sensor.daily_summaries = (filtered_summaries.len() > 0).then_some(filtered_summaries);
        }
    }

    if let Some(end) = end_timestamp {
        let mut filtered_logs: Vec<TemperatureLog> = Vec::new();
        match sensor.logs {
            Some(logs) => {
                for log in logs {
                    if log.timestamp <= end {
                        filtered_logs.push(log);
                    }
                }
                if filtered_logs.len() > 0 {
                    sensor.logs = Some(filtered_logs);
                } else {
                    sensor.logs = None;
                }
            }
            None => {}
        };
        let mut filtered_breaches: Vec<TemperatureBreach> = Vec::new();
        match sensor.breaches {
            Some(breaches) => {
                for breach in breaches {
                    if breach.end_timestamp <= end {
                        // keep if end of breach is before end timestamp
                        filtered_breaches.push(breach);
                    } else if breach.start_timestamp <= end {
                        // if end of breach is after end timestamp
                        filtered_breaches.push(breach); // keep if start of breach is before end timestamp
                    }
                }
                if filtered_breaches.len() > 0 {
                    sensor.breaches = Some(filtered_breaches);
                } else {
                    sensor.breaches = None;
                }
            }
            None => {}
        };
        if let Some(daily_summaries) = sensor.daily_summaries {
            let filtered_summaries: Vec<DailySummary> = daily_summaries
                .into_iter()
                .filter(|daily_summary| daily_summary.date <= end.date())
                .collect();
            sensor.daily_summaries = (filtered_summaries.len() > 0).then_some(filtered_summaries);
        }
    }

    if cfg!(debug_assertions) {
        // Generate output file for debugging/reference
        let output_path = "sensor_".to_owned() + &sensor.serial + "_filtered_output.txt";
        if let Some(mut output) = File::create(&output_path).ok() {
            if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
                log::info!(
                    "Filtered output from {:?} - {:?} to: {}",
                    start_timestamp,
                    end_timestamp,
                    &output_path
                );
            }
        }
    }

    return sensor;
}

#[cfg(target_os = "macos")]
fn sensor_volume_paths() -> Vec<String> {
    let mut volume_list: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir("/Volumes") {
        // loop over folders in Volumes
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(txt_file_path) = entry.path().to_str() {
                        volume_list.push(txt_file_path.to_string())
                    }
                }
            }
        }
    }
    volume_list
}

#[cfg(target_os = "android")]
fn sensor_volume_paths() -> Vec<String> {
    let mut volume_list: Vec<String> = Vec::new();

    if let Ok(entries) = fs::read_dir("/mnt/media_rw") {
        // loop over mounted media folders
        for entry in entries {
            if let Ok(entry) = entry {
                if entry.path().is_dir() {
                    if let Some(txt_file_path) = entry.path().to_str() {
                        volume_list.push(txt_file_path.to_string())
                    }
                }
            }
        }
    }
    volume_list
}

#[cfg(any(target_os = "windows", target_os = "linux"))]
fn sensor_volume_paths() -> Vec<String> {
    let mut volume_list: Vec<String> = Vec::new();

    match drive_list() {
        Err(err) => log::error!("No drives found: {}", err),
        Ok(drives) => {
            for drive_index in 0..drives.len() {
                // loop over all detected drives

                let mount_points = &drives[drive_index].mountpoints;
                for partition_index in 0..mount_points.len() {
                    // loop over partitions
                    let mount_point = &mount_points[partition_index];

                    if mount_point.totalBytes < Some(8 * 1024 * 1024 * 1024) {
                        // possible USB drive if < 8 GB
                        volume_list.push(mount_point.path.clone());
                    }
                }
            }
        }
    }

    volume_list
}

fn sensor_file_list() -> Vec<String> {
    let mut file_list: Vec<String> = Vec::new();

    for volume_root in sensor_volume_paths() {
        // loop over volumes

        if let Ok(entries) = fs::read_dir(&volume_root) {
            // loop over files in the volume root
            for entry in entries {
                if let Ok(entry) = entry {
                    if let Some(extension) = entry.path().extension() {
                        if extension == "txt" {
                            // might be a Berlinger sensor txt file
                            if let Some(txt_file_path) = entry.path().to_str() {
                                let pdf_file_path = txt_file_path.replace(".txt", ".pdf");

                                if Path::new(&pdf_file_path).exists() {
                                    // but only if it has a matching PDF
                                    file_list.push(txt_file_path.to_string())
                                }
                            }
                        };
                        if extension == "csv" {
                            // might be a LogTag sensor CSV file
                            if let Some(txt_file_path) = entry.path().to_str() {
                                file_list.push(txt_file_path.to_string())
                            }
                        };
                    }
                }
            }
        }
    }

    file_list
}

pub fn sensor_serial_from_file_path(txt_file_path: &str) -> Option<String> {
    let mut valid_serial = false;
    let mut serial = "";
    let file_path = Path::new(&txt_file_path);

    if file_path.exists() {
        if let Some(os_file_name) = file_path.file_name() {
            if let Some(file_name) = os_file_name.to_str() {
                let elements: Vec<&str> = file_name.split("_").collect();

                match sensor_type_from_filename(&txt_file_path) {
                    SensorType::Berlinger => {
                        serial = elements[0];
                    }

                    SensorType::LogTag => {
                        serial = elements[1];
                    }
                }
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
///
/// For Berlinger sensors, it expects to find a serial_xxxxx.txt file in the root folder
/// together with a matching PDF file (USB drives can have multiple pairs of files).
///
/// For LogTag sensors, it expects to find a LogTag_serial_xxxxx.csv file in the root folder
///
pub fn read_sensor_serials() -> Option<Vec<String>> {
    let mut serial_list: Vec<String> = Vec::new();

    for txt_file_path in sensor_file_list() {
        if let Some(serial) = sensor_serial_from_file_path(&txt_file_path) {
            serial_list.push(serial)
        }
    }

    if serial_list.len() > 0 {
        Some(serial_list)
    } else {
        None
    }
}

/// Returns all sensors found from currently mounted USB drives up to 8GB capacity
/// (-> any USB drive containing sensor files if you don't have a physical sensor).
///
/// For Berlinger sensors, it expects to find a serial_xxxxx.txt file in the root folder
/// together with a matching PDF file (USB drives can have multiple pairs of files).
///
/// For LogTag sensors, it expects to find a LogTag_serial_xxxxx.csv file in the root folder
///
pub fn read_sensors_from_usb() -> Option<Vec<Sensor>> {
    let mut sensors: Vec<Sensor> = Vec::new();

    for txt_file_path in sensor_file_list() {
        match sensor_type_from_filename(&txt_file_path) {
            SensorType::Berlinger => {
                if let Some(sensor) = berlinger::read_sensor_from_file(&txt_file_path) {
                    sensors.push(sensor.clone())
                };
            }

            SensorType::LogTag => {
                if let Some(sensor) = logtag::read_sensor_from_file(&txt_file_path) {
                    sensors.push(sensor.clone())
                };
            }
        }
    }

    if sensors.len() > 0 {
        Some(sensors)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn test_sample_core() {
        let sensor = sample_sensor();
        assert_eq!(sensor.serial, "reg 1234");
        assert!(sensor.breaches.is_some());
        assert!(sensor.logs.is_some());
        assert!(sensor.configs.is_some());
    }

    #[test]
    fn test_sample_breach() {
        let sensor = sample_sensor();
        let start_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:04:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:17:00", "%Y-%m-%d %H:%M:%S").unwrap();
        if let Some(breaches) = sensor.breaches {
            assert_eq!(breaches[0].start_timestamp, start_timestamp); // start of hot breach
            assert_eq!(breaches[1].end_timestamp, end_timestamp); // end of cold breach
        }
    }

    #[test]
    fn test_sample_log() {
        let sensor = sample_sensor();
        let start_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:04:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:17:00", "%Y-%m-%d %H:%M:%S").unwrap();
        if let Some(logs) = sensor.logs {
            assert_eq!(logs[4].timestamp, start_timestamp); // start of hot breach
            assert_eq!(logs[17].timestamp, end_timestamp); // end of cold breach
        }
    }

    #[test]
    fn test_sample_filter_breach() {
        let start_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:07:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:15:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let sensor = filter_sensor(sample_sensor(), Some(start_timestamp), Some(end_timestamp));
        if let Some(breaches) = sensor.breaches {
            assert_eq!(breaches[0].start_timestamp, start_timestamp); // start of hot breach changed
            assert_eq!(breaches[1].end_timestamp, end_timestamp); // end of cold breach changed
        }
    }

    #[test]
    fn test_sample_filter_log() {
        let start_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:07:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let end_timestamp =
            NaiveDateTime::parse_from_str("2023-05-23 13:15:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let sensor = filter_sensor(sample_sensor(), Some(start_timestamp), Some(end_timestamp));
        if let Some(logs) = sensor.logs {
            assert_eq!(logs[0].timestamp, start_timestamp); // start of hot breach changed
            assert_eq!(logs[8].timestamp, end_timestamp); // end of cold breach changed
        }
    }

    #[test]
    fn test_fridgetag_daily_summaries() {
        let sensor = read_sensor_file("data/FridgeTag 2/130400191544_202304201514.txt").unwrap();
        let daily_summaries = sensor.daily_summaries.unwrap();

        assert_eq!(daily_summaries.len(), 4);
        let daily_summary = &daily_summaries[3];
        assert_eq!(
            daily_summary.date,
            NaiveDate::from_ymd_opt(2023, 4, 20).unwrap()
        );
        assert_eq!(daily_summary.minimum_temperature, Some(2.0));
        assert_eq!(daily_summary.maximum_temperature, Some(8.0));
        assert_eq!(daily_summary.average_temperature, Some(4.6));
        assert_eq!(daily_summary.cold_duration, Some(Duration::zero()));
        assert_eq!(daily_summary.hot_duration, Some(Duration::zero()));
        assert_eq!(daily_summary.cold_threshold, Some(-0.5));
        assert_eq!(daily_summary.hot_threshold, Some(8.0));

        // min & max are still returned as logs
        assert_eq!(sensor.logs.unwrap().len(), 8);
    }

    #[test]
    fn test_fridgetag_alarms_are_consecutive() {
        let sensor = read_sensor_file("data/FridgeTag 2/130400191542_202302140605.txt").unwrap();

        let breaches = sensor.breaches.unwrap();
        assert!(breaches
            .iter()
            .all(|breach| breach.breach_type == BreachType::HotConsecutive
                || breach.breach_type == BreachType::ColdConsecutive));
        let configs = sensor.configs.unwrap();
        assert!(configs
            .iter()
            .all(|config| config.breach_type == BreachType::HotConsecutive
                || config.breach_type == BreachType::ColdConsecutive));

        // the cumulative time out of range is in the daily summaries, alarm or not
        let daily_summaries = sensor.daily_summaries.unwrap();
        let last_day = daily_summaries.last().unwrap();
        assert_eq!(last_day.date, NaiveDate::from_ymd_opt(2023, 2, 14).unwrap());
        assert_eq!(last_day.hot_duration, Some(Duration::minutes(366)));
    }

    #[test]
    fn test_fridgetag_filter_daily_summaries() {
        let sensor = read_sensor_file("data/FridgeTag 2/130400191544_202304201514.txt").unwrap();
        let start_timestamp =
            NaiveDateTime::parse_from_str("2023-04-19 15:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let sensor = filter_sensor(sensor, Some(start_timestamp), None);

        // the day being recorded at the start timestamp is kept, and the day before in case
        // the start timestamp is in another time zone
        let daily_summaries = sensor.daily_summaries.unwrap();
        assert_eq!(daily_summaries.len(), 3);
        assert_eq!(
            daily_summaries[0].date,
            NaiveDate::from_ymd_opt(2023, 4, 18).unwrap()
        );

        let end_timestamp =
            NaiveDateTime::parse_from_str("2023-04-18 09:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
        let sensor = filter_sensor(
            Sensor {
                daily_summaries: Some(daily_summaries),
                ..sensor
            },
            None,
            Some(end_timestamp),
        );
        assert_eq!(sensor.daily_summaries.unwrap().len(), 1);
    }

    #[test]
    fn test_fridgetag_daily_summary_durations() {
        // hot & cold time out of range are read from the right alarms
        let sensor = read_sensor_file("data/FridgeTag 2L/130500109088_202206081014.txt").unwrap();
        let daily_summaries = sensor.daily_summaries.unwrap();
        let daily_summary = daily_summaries
            .iter()
            .find(|daily_summary| {
                daily_summary.date == NaiveDate::from_ymd_opt(2022, 6, 6).unwrap()
            })
            .unwrap();
        assert_eq!(daily_summary.cold_duration, Some(Duration::minutes(753)));
        assert_eq!(daily_summary.hot_duration, Some(Duration::minutes(408)));
    }

    #[test]
    fn test_qtag_has_no_daily_summaries() {
        let sensor =
            read_sensor_file("data/QTag CLm doc LR/RCPJ00010_03_202304201359.txt").unwrap();
        assert!(sensor.daily_summaries.is_none());
    }
}

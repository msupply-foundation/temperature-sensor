use chrono::{Duration, NaiveDateTime};

#[derive(Debug, Clone)]
/// Define the types encountered when parsing a sensor txt file (e.g. Berlinger).
pub enum SensorFieldType {
    Text(String),
    Float(f64),
    Integer(i64),
    Duration(Duration),
    Boolean(bool),
    Timestamp(NaiveDateTime),
}

#[derive(Debug, Clone)]
/// Define the four types of breach.
pub enum BreachType {
    HotConsecutive,
    ColdConsecutive,
    HotCumulative,
    ColdCumulative,
}

#[derive(Debug, Clone)]
/// Define the sensor types supported.
pub enum SensorType {
    Berlinger, // only Berlinger so far
}

#[derive(Debug, Clone)]
/// Define the structure used to capture a temperature log.
pub struct TemperatureLog {
    pub temperature: f64,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone)]
/// Define the structure used to capture a breach config.
pub struct TemperatureBreachConfig {
    pub breach_type: BreachType,
    pub maximum_temperature: f64, // breach if temperature > maximum_temperature
    pub minimum_temperature: f64, // breach if temperature < minimum_temperature
    pub duration: Duration,
}

#[derive(Debug, Clone)]
/// Define the structure used to capture a temperature breach.
pub struct TemperatureBreach {
    pub breach_type: BreachType,
    pub start_timestamp: NaiveDateTime,
    pub end_timestamp: NaiveDateTime,
    pub duration: Duration, // equals (end_timestamp - start_timestamp) for consecutive breaches, but more for cumulative ones
    pub acknowledged: bool,
}

#[derive(Debug, Clone)]
/// Define the structure used to capture sensor details (incomplete).
pub struct Sensor {
    pub sensor_type: SensorType,
    pub serial: String,
    pub name: String,
    pub last_connected_timestamp: Option<NaiveDateTime>,
    pub log_interval: Option<Duration>,
    pub breaches: Option<Vec<TemperatureBreach>>,
    pub configs: Option<Vec<TemperatureBreachConfig>>,
    pub logs: Option<Vec<TemperatureLog>>,
}

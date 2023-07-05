use chrono::{Duration, NaiveDateTime};

#[derive(Debug, Clone)]
// define the types encountered when parsing a sensor text file (e.g. Berlinger)
pub enum SensorFieldType {
    Text(String),
    Float(f64),
    Integer(i64),
    Duration(Duration),
    Boolean(bool),
    Timestamp(NaiveDateTime),
}

#[derive(Debug, Clone)]
// define the four types of breach
pub enum BreachType {
    HotConsecutive,
    ColdConsecutive,
    HotCumulative,
    ColdCumulative,
}

#[derive(Debug, Clone)]
// define the sensor types supported
pub enum SensorType {
    Berlinger,
}

#[derive(Debug, Clone)]
// define the structure used to capture a temperature log
pub struct TemperatureLog {
    pub temperature: f64,
    pub timestamp: NaiveDateTime,
}

#[derive(Debug, Clone)]
// define the structure used to capture a breach config
pub struct TemperatureBreachConfig {
    pub breach_type: BreachType,
    pub maximum_temperature: f64,
    pub minimum_temperature: f64,
    pub duration: Duration,
}

#[derive(Debug, Clone)]
// define the structure used to capture a temperature breach
pub struct TemperatureBreach {
    pub breach_type: BreachType,
    pub start_timestamp: NaiveDateTime,
    pub end_timestamp: NaiveDateTime,
    pub duration: Duration,
    pub acknowledged: bool,
}

#[derive(Debug, Clone)]
// define the structure used to capture sensor details (incomplete)
pub struct Sensor {
    pub sensor_type: SensorType,
    pub registration: String,
    pub name: String,
    pub last_connected_timestamp: Option<NaiveDateTime>,
    pub log_interval: Option<Duration>,
    pub breaches: Option<Vec<TemperatureBreach>>,
    pub configs: Option<Vec<TemperatureBreachConfig>>,
    pub logs: Option<Vec<TemperatureLog>>,
}

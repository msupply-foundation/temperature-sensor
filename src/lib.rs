use chrono::{NaiveDateTime, Duration};

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
    pub timestamp : NaiveDateTime,
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

pub fn sample_sensor() -> Sensor {
    
    let config_cold_consecutive = TemperatureBreachConfig { 
        breach_type: BreachType::ColdConsecutive,
        maximum_temperature: 8.0, 
        minimum_temperature: -273.0, 
        duration: Duration::seconds(240) };

    let config_hot_consecutive = TemperatureBreachConfig { 
        breach_type: BreachType::HotConsecutive,
        maximum_temperature: 100.0, 
        minimum_temperature: 2.0, 
        duration: Duration::seconds(300) };

    let temperature_values = vec![
        3.5,4.0,5.0,7.5, // ok
        8.8,9.2,8.7,9.1,8.4,8.2,8.1, //hot
        7.9,3.2, // ok
        1.2,1.3,0.4,-0.2,0.7, // cold
        2.5 // ok
        ];

    let mut temperature_timestamp = NaiveDateTime::parse_from_str("2023-05-23 13:00:00", "%Y-%m-%d %H:%M:%S").unwrap();
    let interval = Duration::minutes(1);
    let hot_start_timestamp = temperature_timestamp + interval*4;
    let hot_end_timestamp = temperature_timestamp + interval*10;
    let hot_duration = hot_end_timestamp - hot_start_timestamp;//interval*(10-4);
    let cold_start_timestamp = temperature_timestamp + interval*13;
    let cold_end_timestamp = temperature_timestamp + interval*17;
    let cold_duration = cold_end_timestamp - cold_start_timestamp;//interval*(17-13);

    let temperature_iterator = temperature_values.iter();
    let mut temperature_logs: Vec<TemperatureLog> = Vec::new();

    for temperature_value in temperature_iterator {
         temperature_logs.push(TemperatureLog{temperature: *temperature_value, timestamp: temperature_timestamp});         temperature_timestamp = temperature_timestamp+interval;
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

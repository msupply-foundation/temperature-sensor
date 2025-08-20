use chrono::Duration;
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let mut sensor: temperature_sensor::common::Sensor;
    let mut start_timestamp = None;

    if args.len() > 1 {
        // try specified file name
        let file_path = args[1].trim();

        sensor = temperature_sensor::read_sensor_file(&file_path)?;
        if let Some(timestamp) = sensor.last_connected_timestamp {
            start_timestamp = Some(timestamp - Duration::days(3)); // go back from 3 days
        }

        temperature_sensor::filter_sensor(sensor.clone(), start_timestamp, None);
    } else {
        // read from USB
        let sensor_serials = temperature_sensor::read_connected_serials()?;

        for sensor_serial in sensor_serials {
            sensor = temperature_sensor::read_sensor(&sensor_serial)?;

            if let Some(timestamp) = sensor.last_connected_timestamp {
                start_timestamp = Some(timestamp - Duration::days(3)); // go back from 3 days
            }

            temperature_sensor::filter_sensor(sensor, start_timestamp, None);
        }
    }

    Ok(())
}

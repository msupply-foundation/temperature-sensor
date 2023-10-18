use std::error::Error;
use std::env;
use chrono::Duration;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn Error>> {

    let args: Vec<String> = env::args().collect();
    let mut sensor: temperature_sensor::common::Sensor;
    let mut start_timestamp = None;
    let mut sensor_serials: Vec<String> = Vec::new();

    if args.len() > 1 { // try specified file name
        sensor = temperature_sensor::read_sensor_file(args[1].trim())?;
        sensor_serials.push(sensor.serial);
    } else { // read from USB
        sensor_serials = temperature_sensor::read_connected_serials()?;
    }

    for sensor_serial in sensor_serials {

        sensor = temperature_sensor::read_sensor(&sensor_serial)?;

        if let Some(timestamp) = sensor.last_connected_timestamp {
            start_timestamp = Some(timestamp - Duration::days(3)); // go back from 3 days
        }

        sensor = temperature_sensor::filter_sensor(sensor, start_timestamp, None);
        let output_path = "sensor_".to_owned() + &sensor.serial + "_filtered_output.txt";
        
        if let Some(mut output) = File::create(&output_path).ok() {   
            if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
                println!("Filtered output with start time {:?} to: {}", start_timestamp, &output_path);
            }
        }
    }
    
    Ok(())
}

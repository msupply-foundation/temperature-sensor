use std::error::Error;
use std::env;
use chrono::Duration;
use rand::Rng;
use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn Error>> {

    let args: Vec<String> = env::args().collect();
    let mut sensor: temperature_sensor::common::Sensor;
    let mut start_timestamp = None;
    let mut end_timestamp =None; 

    if args.len() > 1 { // try specified file name
        sensor = temperature_sensor::read_sensor_file(args[1].trim())?;
    } else { // pick a random file from USB
        let sensor_serials = temperature_sensor::read_connected_serials()?;
        let mut rng = rand::thread_rng();
        let sensor_index: usize = rng.gen_range(0..sensor_serials.len());
        sensor = temperature_sensor::read_sensor(&sensor_serials[sensor_index])?;
    }

    if let Some(timestamp) = sensor.last_connected_timestamp {
        start_timestamp = Some(timestamp - Duration::days(3)); // go back from 3 days
        end_timestamp = Some(timestamp - Duration::hours(12)); // to 12 hours before
    }

    sensor = temperature_sensor::filter_sensor(sensor,start_timestamp, end_timestamp);
    let output_path = "sensor_".to_owned() + &sensor.serial + "_filtered_output.txt";
    if let Some(mut output) = File::create(&output_path).ok() {   
        if write!(output, "{}", format!("{:?}\n\n", sensor)).is_ok() {
            println!("Filtered output with start/end time {:?} - {:?} to: {}",start_timestamp,end_timestamp, &output_path);
        }
    }
    
    Ok(())
}

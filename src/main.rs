use std::error::Error;
use std::env;

fn main() -> Result<(), Box<dyn Error>> {

    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        let _single_sensor = temperature_sensor::read_sensor_file(args[1].trim())?;
    } else {
        let _sensor_serials = temperature_sensor::read_connected_serials()?;
        let _sensors = temperature_sensor::read_connected_sensors()?;
    };
    
    Ok(())
}

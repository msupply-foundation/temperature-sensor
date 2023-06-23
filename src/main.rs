pub mod berlinger;

use std::env;
use std::fs::File;
use std::io::{Write, Result};
use std::path::Path;

fn main() -> Result<()> {

    let args: Vec<String> = env::args().collect();

    println!("Args: {:?}",args);

    if args.len() > 1 {
        
        let input_path = &args[1];
        println!("Input: {}",input_path);

        if Path::new(input_path).exists() {
            
            let path_elements: Vec<&str> = input_path.split("\\").collect();
            let path_length = path_elements.len();
            let output_path = "sensor_".to_owned()+path_elements[path_length-1];

            let sensor = berlinger::read_sensor_file(input_path);
            let mut output = File::create(output_path.clone())?;
            write!(output,"{}", format!("{:?}\n\n",sensor));
            println!("Output to: {}",output_path);

        } else {
            println!("Input file doesn't exist");
        }
    }

    // Always generate sample
    let sample_sensor = temperature_sensor::sample_sensor();
    let sample_path = "Sample.txt";
    let mut output = File::create(sample_path)?;
    write!(output,"{}", format!("{:?}\n\n",sample_sensor))
}

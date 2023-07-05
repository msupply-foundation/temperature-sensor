pub mod berlinger;
pub mod common;

use std::io::Result;

fn main() -> Result<()> {
    temperature_sensor::read_sensor()
}

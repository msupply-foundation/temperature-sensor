# temperature-sensor
Development crate for processing temperature sensors (initially Berlinger FridgeTags)

Sample data files grouped in the data/<sensor type> folders e.g.:

cargo run "data\FridgeTag 2\130400191544_202304201514.txt"

This will parse the sensor text file and generate a processed version in the root folder if you're running in dev mode

Tested for QTag data, and for the 3 FridgeTag variants I have

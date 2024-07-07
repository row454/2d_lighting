use std::env;

use texture_packer::Config;


fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
	
	let config = Config::build(&args)?;


	

	texture_packer::run(config).map_err(|err| err.to_string())?;
	Ok(())
}

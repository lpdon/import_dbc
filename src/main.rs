use std::env;
use std::process;

use import_dbc::Config;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsings args: {}", err);
        process::exit(1);
    });

    if let Err(e) = import_dbc::run(config) {
        eprintln!("Application error: {}", e);
        process::exit(1);
    }
}

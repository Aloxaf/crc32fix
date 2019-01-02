use clap::{load_yaml, App};
use crc32fix::*;
use std::process;

fn main() {
    let yaml = load_yaml!("../cli.yml");
    let matches = App::from_yaml(yaml).get_matches();

    let inputfile = matches.value_of("inputfile").unwrap();
    let outputfile = matches.value_of("output").unwrap_or("output.png");

    let mut file = PngFile::open(inputfile).unwrap_or_else(|err| {
        eprintln!("failed to open input file: {}", err);
        process::exit(1);
    });

    if let Some((width, height)) = file.try_fix() {
        println!("FOUND! width: {} height: {}", width, height);
        file.save(outputfile).unwrap_or_else(|err| {
            eprintln!("failed to save: {}", err);
            process::exit(1);
        })
    } else {
        eprintln!("not found! : (");
        process::exit(1);
    }
}

use std::env;
use std::process;

use imagetools::PNGFile;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        usage(&args[0]);
        process::exit(1);
    }

    let in_file = &args[1];
    let out_file = &args[2];

    let png_file = PNGFile::from_file(in_file).unwrap_or_else(|err| {
        eprintln!("Could not load {}: {}", in_file, err);
        process::exit(2);
    });

    // Debug - testing Display
    for chunk in png_file.get_chunks() {
        println!("{}\n", chunk);
    }

    png_file.write(out_file).unwrap_or_else(|err| {
        eprintln!("Could not write {}: {}", out_file, err);
        process::exit(3);
    });
}

fn usage(name: &str) {
    println!(
        "usage: {} in_file out_file\n\
         \tin_file\tThe name of the input file\n\
         \tout_file\tThe name of the output file.\n",
        name
    )
}

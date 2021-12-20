use std::{
    fs::File,
    io::{BufRead, BufReader},
};

fn main() {
    set_build_env();
}

fn set_build_env() {
    let path = "build.env";
    println!("cargo:rerun-if-changed={}", path);
    if let Ok(file) = File::open(path) {
        for (key, value) in BufReader::new(file)
            .lines()
            .map(|line| line.unwrap())
            .collect::<Vec<_>>()
            .iter()
            .flat_map(|line| line.split_once('='))
        {
            println!("cargo:rustc-env={}={}", key, value);
        }
    }
}

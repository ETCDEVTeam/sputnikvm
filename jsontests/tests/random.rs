#![allow(non_snake_case)]

extern crate jsontests;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

use serde_json::Value;
use jsontests::test_transaction;

use std::fs;
use std::fs::File;
use std::path::PathBuf;
use std::collections::HashMap;
use std::io::Read;

#[test]
fn ramdomTests() {
    let files_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("res/files/RandomTests");

    let paths = fs::read_dir(&files_path).unwrap();
    for path in paths {
        let path = path.unwrap().path();

        println!("Running RandomTests::{}", path.file_name().unwrap()
            .to_str().unwrap());

        let mut file = File::open(path).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();

        let tests: HashMap<String, serde_json::Value> = serde_json::from_str(&*data).unwrap();
        for (name, value) in &tests {
            println!("\t test {} ... ", name);
            assert_eq!(test_transaction(name, value, true), true);
        }
    }
}

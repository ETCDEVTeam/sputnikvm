#![allow(non_snake_case)]

extern crate jsontests;
extern crate serde_json;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use std::fs;

use jsontests::test_transaction;

#[test]
fn vmArithTest() {
    let paths = fs::read_dir("./res/VMTests/vmArithmeticTest").unwrap();
    for path in paths {
        let mut file = File::open(path.unwrap().path()).unwrap();
        let mut data = String::new();
        file.read_to_string(&mut data).unwrap();
        let test_data: HashMap<String, serde_json::Value> = serde_json::from_str(&*data).unwrap();
        for (opcode, test_datum) in test_data.iter() {
            println!("opcode: {}", opcode);
            assert_eq!(test_transaction(opcode, test_datum, true), true);
        }
    }
}

#![allow(non_snake_case)]

extern crate jsontests;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

use serde_json::Value;
use jsontests::test_transaction;

lazy_static! {
    static ref TESTS: Value =
        serde_json::from_str(include_str!("../res/files/vmInputLimitsLight.json")).unwrap();
}

#[test]
fn inputLimitsLight() {
    for (name, value) in TESTS.as_object().unwrap().iter() {
        print!("\t{} ... ", name);
        if !test_transaction(name, value, true) {
            panic!("test inputLimitsLight::{} failed", name);
        }
    }
}

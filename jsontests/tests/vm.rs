#![allow(non_snake_case)]

extern crate jsontests;
extern crate serde_json;
#[macro_use]
extern crate lazy_static;

use serde_json::Value;
use jsontests::test_transaction;

lazy_static! {
    static ref TESTS: Value =
        serde_json::from_str(include_str!("../res/files/vmtests.json")).unwrap();
}

#[test] fn arith() { assert_eq!(test_transaction("arith", &TESTS["arith"], true), Ok(true)); }
#[test] fn boolean() { assert_eq!(test_transaction("boolean", &TESTS["boolean"], true), Ok(true)); }
#[test] fn mktx() { assert_eq!(test_transaction("mktx", &TESTS["mktx"], true), Ok(true)); }
#[test] fn suicide() { assert_eq!(test_transaction("suicide", &TESTS["suicide"], true), Ok(true)); }

#[test] fn all_tests_included() {
    for (testname, _) in TESTS.as_object().unwrap().iter() {
        println!("#[test] fn {}() {{ assert_eq!(test_transaction({:?}, &TESTS[{:?}], true), Ok(true)); }}", testname, testname, testname)
    }
    assert_eq!(TESTS.as_object().unwrap().len(), 4);
}

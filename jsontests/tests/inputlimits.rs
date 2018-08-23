#![allow(non_snake_case)]
#![allow(unused)]

#[macro_use]
extern crate jsontests_derive;
extern crate jsontests;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmInputLimits"]
#[test_with = "jsontests::util::run_test"]
struct InputLimits;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmInputLimitsLight"]
#[test_with = "jsontests::util::run_test"]
struct InputLimitsLight;


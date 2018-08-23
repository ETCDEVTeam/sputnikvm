#![cfg_attr(feature = "bench", feature(test))]
#![allow(non_snake_case)]
#![allow(unused)]

#[macro_use]
extern crate jsontests_derive;
extern crate jsontests;

#[cfg(feature = "bench")]
extern crate test;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmArithmeticTest"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct Arithmetic;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmBitwiseLogicOperation"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct BitwiseLogicOperation;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmBlockInfoTest"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct BlockInfo;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmEnvironmentalInfo"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct VmInverontemtalInfo;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmIOandFlowOperations"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct VmIOandFlowOperations;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmLogTest"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct Log;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmPushDupSwapTest"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct PushDupSwap;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmRandomTest"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct Random;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmSha3Test"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct Sha3;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmSystemOperations"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct SystemOperations;

#[derive(JsonTests)]
#[directory = "jsontests/res/files/vmTests"]
#[test_with = "jsontests::util::run_test"]
#[cfg_attr(feature = "bench", bench_with = "jsontests::util::run_bench")]
struct VM;

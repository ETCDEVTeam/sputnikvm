#![allow(dead_code)]

use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::values::IntValue;

#[derive(Debug, Singleton)]

pub struct EvmConstants {
    gas_max: IntValue,
}

unsafe impl Sync for EvmConstants {}
unsafe impl Send for EvmConstants {}

impl SingletonInit for EvmConstants {
    type Init = Context;
    
    fn init(context: &Context) -> Self {
        EvmConstants {
            gas_max : context.i64_type().const_int(std::i64::MAX as u64, false)
        }
    }
}

impl EvmConstants {
    pub fn get_gas_max(&self) -> IntValue {
        self.gas_max
    }
}

#[test]
fn test_evmconstants() {
    let context = Context::create();
    let evm_constants_singleton = EvmConstants::get_instance(&context);

    let max_g = evm_constants_singleton.get_gas_max();
    assert!(max_g.is_const());
    assert_eq!(max_g.get_zero_extended_constant(), Some(std::i64::MAX as u64));
}



#![allow(dead_code)]

use inkwell::context::Context;
use inkwell::values::IntValue;
use singletonum::{Singleton, SingletonInit};

#[derive(Debug, Singleton)]
/// Singleton structure containing basic EVM gas constants.
pub struct EvmConstants {
    /// The maximum gas value for the VM.
    gas_max: IntValue,
    /// The zero gas value for the VM.
    i64_zero: IntValue,
}

unsafe impl Sync for EvmConstants {}
unsafe impl Send for EvmConstants {}

impl SingletonInit for EvmConstants {
    type Init = Context;

    fn init(context: &Context) -> Self {
        EvmConstants {
            gas_max: context.i64_type().const_int(std::i64::MAX as u64, false),
            i64_zero: context.i64_type().const_int(0, false),
        }
    }
}

impl EvmConstants {
    /// Returns the maximum gas value for the VM.
    pub fn get_gas_max(&self) -> IntValue {
        self.gas_max
    }
    /// Returns the zero (Out-Of-Gas) value for the VM
    pub fn get_i64_zero(&self) -> IntValue {
        self.i64_zero
    }
}

#[test]
fn test_evmconstants() {
    let context = Context::create();
    let evm_constants_singleton = EvmConstants::get_instance(&context);

    let max_g = evm_constants_singleton.get_gas_max();
    assert!(max_g.is_const());
    assert_eq!(
        max_g.get_zero_extended_constant(),
        Some(std::i64::MAX as u64)
    );

    let i64_zero = evm_constants_singleton.get_i64_zero();
    assert!(i64_zero.is_const());
    assert_eq!(i64_zero.get_zero_extended_constant(), Some(0));
}

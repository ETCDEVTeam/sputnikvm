#![allow(dead_code)]

use inkwell::context::Context;
use inkwell::types::IntType;
use inkwell::types::PointerType;
use inkwell::types::VoidType;
use inkwell::AddressSpace;
use singletonum::{Singleton, SingletonInit};

#[derive(Debug, Singleton)]

/// Singleton structure declaring EVM types in LLVM format.
pub struct EvmTypes {
    /// A basic EVM word. 256 bits.
    word_type: IntType,
    /// A basic word pointer type.
    word_ptr_type: PointerType,
    /// A boolean type. 1 bits.
    bool_type: IntType,
    /// A size type. 64 bits.
    size_type: IntType,
    /// A gas counter type. Same as `size_type`.
    gas_type: IntType,
    /// A pointer to a gas counter type.
    gas_ptr_type: PointerType,
    /// A single byte type.
    byte_type: IntType,
    /// A byte pointer type.
    byte_ptr_type: PointerType,
    /// A void type.
    void_type: VoidType,
    /// A contract return code type. 32 bits.
    contract_ret_type: IntType,
    /// An address type. 160 bits.
    address_type: IntType,
    /// An address pointer type.
    address_ptr_type: PointerType,
}

unsafe impl Sync for EvmTypes {}
unsafe impl Send for EvmTypes {}

impl SingletonInit for EvmTypes {
    type Init = Context;

    fn init(context: &Context) -> Self {
        let word_t = context.custom_width_int_type(256);
        let word_ptr_t = word_t.ptr_type(AddressSpace::Generic);
        let bool_t = context.bool_type();
        let size_t = context.i64_type();
        let gas_t = size_t;
        let gas_ptr_t = gas_t.ptr_type(AddressSpace::Generic);
        let byte_t = context.i8_type();
        let byte_ptr_t = byte_t.ptr_type(AddressSpace::Generic);
        let void_t = context.void_type();
        let contract_ret_t = context.i32_type();
        let address_t = context.custom_width_int_type(160);

        EvmTypes {
            word_type: word_t,
            word_ptr_type: word_ptr_t,
            bool_type: bool_t,
            size_type: size_t,
            gas_type: gas_t,
            gas_ptr_type: gas_ptr_t,
            byte_type: byte_t,
            byte_ptr_type: byte_ptr_t,
            void_type: void_t,
            contract_ret_type: contract_ret_t,
            address_type: address_t,
            address_ptr_type: address_t.ptr_type(AddressSpace::Generic),
        }
    }
}

impl EvmTypes {
    /// Returns the LLVM type for an EVM word.
    pub fn get_word_type(&self) -> IntType {
        self.word_type
    }

    /// Returns the LLVM type for an EVM word pointer.
    pub fn get_word_ptr_type(&self) -> PointerType {
        self.word_ptr_type
    }

    /// Returns the LLVM type for a boolean.
    pub fn get_bool_type(&self) -> IntType {
        self.bool_type
    }

    /// Returns the LLVM type for a size value.
    pub fn get_size_type(&self) -> IntType {
        self.size_type
    }

    /// Returns the LLVM type for a gas counter.
    pub fn get_gas_type(&self) -> IntType {
        self.gas_type
    }

    /// Returns the LLVM type for a gas value pointer.
    pub fn get_gas_ptr_type(&self) -> PointerType {
        self.gas_ptr_type
    }

    /// Returns the LLVM type for a byte.
    pub fn get_byte_type(&self) -> IntType {
        self.byte_type
    }

    /// Returns the LLVM type for a byte pointer.
    pub fn get_byte_ptr_type(&self) -> PointerType {
        self.byte_ptr_type
    }

    /// Returns the LLVM type for void.
    pub fn get_void_type(&self) -> VoidType {
        self.void_type
    }

    /// Returns the LLVM type for a message call's return code.
    pub fn get_contract_return_type(&self) -> IntType {
        self.contract_ret_type
    }

    /// Returns the LLVM type for an Ethereum address.
    pub fn get_address_type(&self) -> IntType {
        self.address_type
    }

    /// Returns the LLVM type for an address pointer.
    pub fn get_address_ptr_type(&self) -> PointerType {
        self.address_ptr_type
    }
}

#[test]
fn test_evmtypes() {
    let context = Context::create();
    let evm_type_singleton = EvmTypes::get_instance(&context);
    assert_eq!(evm_type_singleton.get_word_type().get_bit_width(), 256);

    let evm_word_ptr = evm_type_singleton.get_word_ptr_type();
    assert_eq!(evm_word_ptr.get_address_space(), AddressSpace::Generic);
    assert_eq!(
        evm_word_ptr.get_element_type().into_int_type(),
        context.custom_width_int_type(256)
    );

    assert_eq!(evm_type_singleton.get_bool_type().get_bit_width(), 1);
    assert_eq!(evm_type_singleton.get_size_type(), context.i64_type());
    assert_eq!(evm_type_singleton.get_size_type().get_bit_width(), 64);

    assert_eq!(evm_type_singleton.get_gas_type(), context.i64_type());
    assert_eq!(evm_type_singleton.get_gas_type().get_bit_width(), 64);

    let evm_gas_ptr_t = evm_type_singleton.get_gas_ptr_type();
    assert_eq!(evm_gas_ptr_t.get_address_space(), AddressSpace::Generic);
    assert_eq!(
        evm_gas_ptr_t.get_element_type().into_int_type(),
        context.i64_type()
    );

    assert_eq!(evm_type_singleton.get_byte_type(), context.i8_type());
    assert_eq!(evm_type_singleton.get_byte_type().get_bit_width(), 8);

    let evm_byte_ptr_t = evm_type_singleton.get_byte_ptr_type();
    assert_eq!(evm_byte_ptr_t.get_address_space(), AddressSpace::Generic);
    assert_eq!(
        evm_byte_ptr_t.get_element_type().into_int_type(),
        context.i8_type()
    );

    assert_eq!(evm_type_singleton.get_void_type(), context.void_type());
    assert_eq!(evm_type_singleton.get_void_type().is_sized(), false);

    assert_eq!(
        evm_type_singleton.get_contract_return_type(),
        context.i32_type()
    );

    assert_eq!(evm_type_singleton.get_address_type().get_bit_width(), 160);

    let evm_address_ptr = evm_type_singleton.get_address_ptr_type();
    assert_eq!(evm_address_ptr.get_address_space(), AddressSpace::Generic);
    assert_eq!(
        evm_address_ptr
            .get_element_type()
            .into_int_type()
            .get_bit_width(),
        160
    );
}

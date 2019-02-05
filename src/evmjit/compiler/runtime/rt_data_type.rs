#![allow(dead_code)]

use std::ffi::CString;
use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::AddressSpace;
use evmjit::BasicTypeEnumCompare;

pub const NUM_RUNTIME_DATA_FIELDS: usize = 10;
/// Enum representing the runtime data fields.
pub enum RuntimeDataTypeFields {
    Gas,
    GasPrice,
    CallData,
    CallDataSize,
    Value,
    Code,
    CodeSize,
    Address,
    Sender,
    Depth,
    ReturnData,      // Pointer to return data, used only on RETURN
    ReturnDataSize   // Return data size, used only on RETURN
}

/// Trait mapping a runtime data field enum to an index.
pub trait RuntimeDataFieldToIndex {
    fn to_index(&self) -> usize;
}

impl RuntimeDataFieldToIndex for RuntimeDataTypeFields {
    fn to_index(&self) -> usize {
        match self {
            RuntimeDataTypeFields::Gas => 0,
            RuntimeDataTypeFields::GasPrice => 1,
            RuntimeDataTypeFields::CallData => 2,
            RuntimeDataTypeFields::CallDataSize => 3,
            RuntimeDataTypeFields::Value => 4,
            RuntimeDataTypeFields::Code => 5,
            RuntimeDataTypeFields::CodeSize => 6,
            RuntimeDataTypeFields::Address => 7,
            RuntimeDataTypeFields::Sender => 8,
            RuntimeDataTypeFields::Depth => 9,
            RuntimeDataTypeFields::ReturnData => 2, // We deliberately overlap with CallData
            RuntimeDataTypeFields::ReturnDataSize => 3, // We deliberately overlap with CallDataSize
        }
    }

}

/// Trait mapping a runtime data field enum to a name.
pub trait RuntimeDataFieldToName {
    fn to_name(&mut self) -> &'static str;
}

impl RuntimeDataFieldToName for RuntimeDataTypeFields {
    fn to_name(&mut self) -> &'static str {
        match self {
            RuntimeDataTypeFields::Gas => "msg.gas",
            RuntimeDataTypeFields::GasPrice => "tx.gasprice",
            RuntimeDataTypeFields::CallData => "msg.data.ptr",
            RuntimeDataTypeFields::CallDataSize => "msg.data.size",
            RuntimeDataTypeFields::Value => "msg.value",
            RuntimeDataTypeFields::Code => "code.ptr",
            RuntimeDataTypeFields::CodeSize => "code.size",
            RuntimeDataTypeFields::Address => "msg.address",
            RuntimeDataTypeFields::Sender => "msg.sender",
            RuntimeDataTypeFields::Depth => "msg.depth",
            RuntimeDataTypeFields::ReturnData => "", 
            RuntimeDataTypeFields::ReturnDataSize => "",
        }
    }

}

#[derive(Debug, Singleton)]
/// RuntimeDataType is the struct that the JIT will build to pass
/// arguments from the VM to the contract at runtime.
pub struct RuntimeDataType
{
    /// The LLVM type representing runtime data.
    rt_type: StructType,
    /// The LLVM type representing a runtime data pointer.
    rt_ptr_type: PointerType,
}

unsafe impl Sync for RuntimeDataType {}
unsafe impl Send for RuntimeDataType {}

impl SingletonInit for RuntimeDataType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let size_t = context.i64_type();
        let byte_ptr_t = context.i8_type().ptr_type(AddressSpace::Generic);
        let evm_word_t = context.custom_width_int_type(256);
        let fields = [size_t.into(),      // gas
                      size_t.into(),      // gas price
                      byte_ptr_t.into(),  // calldata
                      size_t.into(),      // calldata size
                      evm_word_t.into(),  // apparent value
                      byte_ptr_t.into(),  // pointer to evm byte code
                      size_t.into(),      // size of evm byte code
                      evm_word_t.into(),  // address
                      evm_word_t.into(),  // caller address
                      size_t.into()];     // call depth
        
        let rt_struct = context.opaque_struct_type("RuntimeData");
        rt_struct.set_body(&fields, false);
        
        RuntimeDataType {
            rt_type: rt_struct,
            rt_ptr_type: rt_struct.ptr_type(AddressSpace::Generic)
        }
    }
}

impl RuntimeDataType {
    /// Returns the LLVM type of runtime data.
    pub fn get_type(&self) -> StructType {
        self.rt_type
    }
    
    /// Returns the LLVM type of a runtime data pointer.
    pub fn get_ptr_type(&self) -> PointerType {
        self.rt_ptr_type
    }

    /// Validates the properties of a runtime data type.
    pub fn is_rt_data_type(a_struct: &StructType) -> bool {
        if !a_struct.is_sized() {
            return false;
        }

        if a_struct.count_fields() != 10 {
            return false;
        }

        if a_struct.is_packed() {
            return false;
        }

        if a_struct.is_opaque() {
            return false;
        }

        if a_struct.get_name() != Some(&*CString::new("RuntimeData").unwrap()) {
            return false;
        }

        let field1 = a_struct.get_field_type_at_index(0).unwrap();

        if !field1.is_int64() {
            return false;
        }

        let field2 = a_struct.get_field_type_at_index(1).unwrap();
        if !field2.is_int64() {
            return false;
        }

        let field3 = a_struct.get_field_type_at_index(2).unwrap();
        if !field3.is_ptr_to_int8() {
            return false;
        }

        let field4 = a_struct.get_field_type_at_index(3).unwrap();
        if !field4.is_int64() {
            return false;
        }

        let field5 = a_struct.get_field_type_at_index(4).unwrap();
        if !field5.is_int256() {
            return false;
        }

        let field6 = a_struct.get_field_type_at_index(5).unwrap();
        if !field6.is_ptr_to_int8() {
            return false;
        }

        let field7 = a_struct.get_field_type_at_index(6).unwrap();
        if !field7.is_int64() {
            return false;
        }

        let field8 = a_struct.get_field_type_at_index(7).unwrap();
        if !field8.is_int256() {
            return false;
        }

        let field9 = a_struct.get_field_type_at_index(8).unwrap();
        if !field9.is_int256() {
            return false;
        }

        let field10 = a_struct.get_field_type_at_index(9).unwrap();
        if !field10.is_int64() {
            return false;
        }

        true
    }

}

#[test]

fn test_data_field_to_index() {
    assert_eq!(RuntimeDataTypeFields::Gas.to_index(), 0);
    assert_eq!(RuntimeDataTypeFields::GasPrice.to_index(), 1);
    assert_eq!(RuntimeDataTypeFields::CallData.to_index(), 2);
    assert_eq!(RuntimeDataTypeFields::CallDataSize.to_index(), 3);
    assert_eq!(RuntimeDataTypeFields::Value.to_index(), 4);
    assert_eq!(RuntimeDataTypeFields::Code.to_index(), 5);
    assert_eq!(RuntimeDataTypeFields::CodeSize.to_index(), 6);
    assert_eq!(RuntimeDataTypeFields::Address.to_index(), 7);
    assert_eq!(RuntimeDataTypeFields::Sender.to_index(), 8);
    assert_eq!(RuntimeDataTypeFields::Depth.to_index(), 9);
    assert_eq!(RuntimeDataTypeFields::ReturnData.to_index(), RuntimeDataTypeFields::CallData.to_index());
    assert_eq!(RuntimeDataTypeFields::ReturnDataSize.to_index(), RuntimeDataTypeFields::CallDataSize.to_index());
}

#[test]

fn test_data_field_to_name() {
    assert_eq!(RuntimeDataTypeFields::Gas.to_name(), "msg.gas");
    assert_eq!(RuntimeDataTypeFields::GasPrice.to_name(), "tx.gasprice");
    assert_eq!(RuntimeDataTypeFields::CallData.to_name(), "msg.data.ptr");
    assert_eq!(RuntimeDataTypeFields::CallDataSize.to_name(), "msg.data.size");
    assert_eq!(RuntimeDataTypeFields::Value.to_name(), "msg.value");
    assert_eq!(RuntimeDataTypeFields::Code.to_name(), "code.ptr");
    assert_eq!(RuntimeDataTypeFields::CodeSize.to_name(), "code.size");
    assert_eq!(RuntimeDataTypeFields::Address.to_name(), "msg.address");
    assert_eq!(RuntimeDataTypeFields::Sender.to_name(), "msg.sender");
    assert_eq!(RuntimeDataTypeFields::Depth.to_name(), "msg.depth");

}

#[test]

fn test_runtime_data_type() {
    let context = Context::create();
    let rt_data_type_singleton = RuntimeDataType::get_instance(&context);
    let rt_struct = rt_data_type_singleton.get_type();

    assert!(RuntimeDataType::is_rt_data_type (&rt_struct));

    // Test for inequality of RuntimeData
    let size_t = context.i64_type();
    let byte_ptr_t = context.i8_type().ptr_type(AddressSpace::Generic);
    let evm_word_t = context.custom_width_int_type(256);
    let fields = [size_t.into(),      // gas
                  size_t.into(),      // gas price
                  byte_ptr_t.into(),  // calldata
                  size_t.into(),      // calldata size
                  evm_word_t.into(),  // apparent value
                  byte_ptr_t.into(),  // pointer to evm byte code
                  size_t.into(),      // size of evm byte code
                  evm_word_t.into(),  // address
                  evm_word_t.into(),  // caller address
                  evm_word_t.into()];     // call depth

    let rt_struct2 = context.opaque_struct_type("RuntimeData");
    rt_struct2.set_body(&fields, false);
    assert!(!RuntimeDataType::is_rt_data_type (&rt_struct2));

    // Test that we have a pointer to RuntimeData

    let rt_struct_ptr = rt_data_type_singleton.get_ptr_type();
    assert!(rt_struct_ptr.get_element_type().is_struct_type());
    assert!(RuntimeDataType::is_rt_data_type (rt_struct_ptr.get_element_type().as_struct_type()));
}

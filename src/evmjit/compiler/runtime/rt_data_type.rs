#![allow(dead_code)]

use std::ffi::CString;
use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::AddressSpace;

const NUM_RUNTIME_DATA_FIELDS: usize = 10;

enum RuntimeDataTypeFields {
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

trait RuntimeDataFieldToIndex {
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
            RuntimeDataTypeFields::ReturnData => 2, // We are deliberately ovelap with CallData
            RuntimeDataTypeFields::ReturnDataSize => 3, // We are deliberately ovelap with CallDataSize
        }
    }

}

trait RuntimeDataFieldToName {
    fn to_name(&mut self, field: RuntimeDataTypeFields) -> &'static str;
}

impl RuntimeDataFieldToName for RuntimeDataTypeFields {
    fn to_name(&mut self, field: RuntimeDataTypeFields) -> &'static str {
        match field {
            RuntimeDataTypeFields::Gas => "msg.gas",
            RuntimeDataTypeFields::GasPrice => "tx.gasprice",
            RuntimeDataTypeFields::CallData => "msg.data.ptr",
            RuntimeDataTypeFields::CallDataSize => "msg.data.size",
            RuntimeDataTypeFields::Value => "msg.value",
            RuntimeDataTypeFields::Code => "code.ptr",
            RuntimeDataTypeFields::CodeSize => "code.size",
            RuntimeDataTypeFields::Address => "msg.address",
            RuntimeDataTypeFields::Sender => "message.sender",
            RuntimeDataTypeFields::Depth => "msg.depth",
            RuntimeDataTypeFields::ReturnData => "", 
            RuntimeDataTypeFields::ReturnDataSize => "",
        }
    }
}

#[derive(Debug, Singleton)]

// RuntimeDataType is the struct that the JIT will build to pass
// arguments from the VM to the contract at runtime

pub struct RuntimeDataType
{
    rt_type: StructType,
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
    pub fn get_type(&self) -> StructType {
        self.rt_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.rt_ptr_type
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

fn test_runtime_data_type() {
    let context = Context::create();
    let rt_data_type_singleton = RuntimeDataType::get_instance(&context);
    let rt_struct = rt_data_type_singleton.get_type();
    assert!(!rt_struct.is_packed());
    assert!(!rt_struct.is_opaque());
    assert!(rt_struct.is_sized());
    assert_eq!(rt_struct.get_name(), Some(&*CString::new("RuntimeData").unwrap()));
    assert_eq!(rt_struct.count_fields(), 10);

    let size_t = context.i64_type();
    let byte_ptr_t = context.i8_type().ptr_type(AddressSpace::Generic);
    let evm_word_t = context.custom_width_int_type(256);
    
    assert_eq!(rt_struct.get_field_types(), &[size_t.into(), size_t.into(),
                                              byte_ptr_t.into(), size_t.into(),
                                              evm_word_t.into(), byte_ptr_t.into(),
                                              size_t.into(), evm_word_t.into(),
                                              evm_word_t.into(), size_t.into()]);
}

#![allow(dead_code)]

#[cfg(test)]
use std::ffi::CString;

use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::AddressSpace;

#[derive(Debug, Singleton)]

pub struct EnvDataType
{
    env_type: StructType,
    env_ptr_type: PointerType,
}

unsafe impl Sync for EnvDataType {}
unsafe impl Send for EnvDataType {}

impl SingletonInit for EnvDataType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let env_t = context.opaque_struct_type("Env");
        
        EnvDataType {
            env_type : env_t,
            env_ptr_type : env_t.ptr_type(AddressSpace::Generic)
        }
    }
}

impl EnvDataType {
    pub fn get_type(&self) -> StructType {
        self.env_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.env_ptr_type
    }
}

#[test]
fn test_env_data_type() {
    let context = Context::create();
    let env_data_type_singleton = EnvDataType::get_instance(&context);
    let env_data_t = env_data_type_singleton.get_type();

    assert!(!env_data_t.is_packed());
    assert!(env_data_t.is_opaque());
    assert!(!env_data_t.is_sized());
    assert_eq!(env_data_t.get_name(), Some(&*CString::new("Env").unwrap()));
    assert_eq!(env_data_t.count_fields(), 0);

    let env_data_ptr_t = env_data_type_singleton.get_ptr_type();
    assert_eq!(env_data_ptr_t.get_address_space(), AddressSpace::Generic);
    assert_eq!(env_data_ptr_t.get_element_type().into_struct_type(), env_data_t);
}

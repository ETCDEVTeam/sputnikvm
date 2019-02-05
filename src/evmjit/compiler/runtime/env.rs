#![allow(dead_code)]

use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::AddressSpace;
use std::ffi::CString;

#[derive(Debug, Singleton)]
/// Internal representation of an EVM environment.
pub struct EnvDataType
{
    /// An EVM environment type.
    env_type: StructType,
    /// An EVM environment pointer type.
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
    /// Returns the LLVM type for an EVM environment.
    pub fn get_type(&self) -> StructType {
        self.env_type
    }
    
    /// Returns the LLVM type for an EVM environment pointer.
    pub fn get_ptr_type(&self) -> PointerType {
        self.env_ptr_type
    }
    
    /// Validates the basic properties of an EVM environment.
    pub fn is_env_data_type(a_struct: &StructType) -> bool {
        if a_struct.count_fields() != 0 {
            return false;
        }

        if a_struct.is_sized() {
            return false;
        }
        
        if a_struct.is_packed() {
            return false;
        }
            
        if !a_struct.is_opaque() {
            return false;
        }
        
        if a_struct.get_name() != Some(&*CString::new("Env").unwrap()) {
            return false;
        }

        return true;
    }
}

#[test]
fn test_env_data_type() {
    let context = Context::create();
    let env_data_type_singleton = EnvDataType::get_instance(&context);
    let env_data_t = env_data_type_singleton.get_type();

    assert!(EnvDataType::is_env_data_type(&env_data_t));
    
    let env_data_ptr_t = env_data_type_singleton.get_ptr_type();
    assert!(env_data_ptr_t.get_element_type().is_struct_type());
    assert!(EnvDataType::is_env_data_type (env_data_ptr_t.get_element_type().as_struct_type()));
}

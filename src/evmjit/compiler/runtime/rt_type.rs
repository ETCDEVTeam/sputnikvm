#![allow(dead_code)]

#[cfg(test)]
use std::ffi::CString;
use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::PointerValue;
use inkwell::values::BasicValueEnum;
use inkwell::AddressSpace;
use super::super::memory::mem_representation::MemoryRepresentationType;
use super::env::EnvDataType;
use super::rt_data_type::RuntimeDataType;

#[derive(Debug, Singleton)]

// RuntimeType is the struct that the JIT will build to pass
// arguments from the VM to the contract at runtime

pub struct RuntimeType
{
    rt_type: StructType,
    rt_ptr_type: PointerType,
}

unsafe impl Sync for RuntimeType {}
unsafe impl Send for RuntimeType {}

impl SingletonInit for RuntimeType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let rt_data_ptr = RuntimeDataType::get_instance(&context).get_ptr_type();
        let env_ptr = EnvDataType::get_instance(&context).get_ptr_type();
        let mem_ptr = MemoryRepresentationType::get_instance(&context).get_type();

        let fields = [rt_data_ptr.into(), env_ptr.into(), mem_ptr.into()];
        let rt_struct = context.opaque_struct_type("Runtime");
        rt_struct.set_body(&fields, false);

        RuntimeType {
            rt_type : rt_struct,
            rt_ptr_type : rt_struct.ptr_type(AddressSpace::Generic)
        }
    }
}

impl RuntimeType {
    pub fn get_type(&self) -> StructType {
        self.rt_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.rt_ptr_type
    }
}

pub struct RuntimeTypeManager {
    m_data_ptr: BasicValueEnum,
    m_mem_ptr: PointerValue,
    m_env_ptr: BasicValueEnum,
}

impl RuntimeTypeManager {
    pub fn new(context: &Context, builder: &Builder) -> RuntimeTypeManager {

        let rt_ptr = RuntimeTypeManager::get_runtime_ptr_with_builder(&context, &builder);
        unsafe {
            let data_p = builder.build_load (builder.build_struct_gep(rt_ptr.into_pointer_value(), 0, ""), "dataPtr");
            assert_eq!(data_p.get_type().into_pointer_type(), RuntimeDataType::get_instance(context).get_ptr_type());

            let mem_p = builder.build_struct_gep(rt_ptr.into_pointer_value(), 2, "mem");
            assert_eq!(mem_p.get_type(), MemoryRepresentationType::get_instance(&context).get_ptr_type());

            let env_p = builder.build_load (builder.build_struct_gep(rt_ptr.into_pointer_value(), 1, ""), "env");
            assert_eq!(env_p.get_type().into_pointer_type(), EnvDataType::get_instance(&context).get_ptr_type());

            RuntimeTypeManager {
                m_data_ptr: data_p,
                m_mem_ptr: mem_p,
                m_env_ptr: env_p,
            }
        }
    }

    fn get_runtime_ptr_with_builder(context: & Context, builder: & Builder) -> BasicValueEnum {
        // The parent of the first basic block is a function
        
        let bb = builder.get_insert_block();
        assert!(bb != None);
        
        let func = bb.unwrap().get_parent();
        assert!(func != None);
        let func_val = func.unwrap();
        
        // The first argument to a function is a pointer to the runtime
        assert!(func_val.count_params() > 0);

        let runtime_ptr = func_val.get_first_param().unwrap();
        assert_eq!(runtime_ptr.get_type().into_pointer_type(), RuntimeType::get_instance(context).get_ptr_type());

        runtime_ptr
    }

}

#[test]

fn test_runtime_type() {
    let context = Context::create();
    let rt_type_singleton = RuntimeType::get_instance(&context);
    let rt_struct = rt_type_singleton.get_type();
    assert!(!rt_struct.is_packed());
    assert!(!rt_struct.is_opaque());
    assert!(rt_struct.is_sized());
    assert_eq!(rt_struct.get_name(), Some(&*CString::new("Runtime").unwrap()));
    assert_eq!(rt_struct.count_fields(), 3);

    let rt_data_ptr = RuntimeDataType::get_instance(&context).get_ptr_type();
    let env_ptr = EnvDataType::get_instance(&context).get_ptr_type();
    let mem_ptr = MemoryRepresentationType::get_instance(&context).get_type();

    assert_eq!(rt_struct.get_field_types(), &[rt_data_ptr.into(), env_ptr.into(), mem_ptr.into()]);
}

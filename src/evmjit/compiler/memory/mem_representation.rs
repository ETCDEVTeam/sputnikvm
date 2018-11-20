#![allow(dead_code)]

use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::PointerValue;
use inkwell::AddressSpace;

#[derive(Debug, Singleton)]

// Internal representation of EVM linear memory

pub struct MemoryRepresentationType {
    memory_type: StructType,
    memory_ptr_type: PointerType,
}

unsafe impl Sync for MemoryRepresentationType {}
unsafe impl Send for MemoryRepresentationType {}

impl SingletonInit for MemoryRepresentationType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let evm_word_t = context.custom_width_int_type(256);
        let size_t = context.i64_type();

        let fields = [evm_word_t.into(),
                      size_t.into(),
                      size_t.into()];

        let mem_struct = context.opaque_struct_type("LinearMemory");
        mem_struct.set_body(&fields, false);
        
        MemoryRepresentationType {
            memory_type: mem_struct,
            memory_ptr_type: mem_struct.ptr_type(AddressSpace::Generic)
        }
    }
}

impl MemoryRepresentationType {    
    pub fn get_type(&self) -> StructType {
        self.memory_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.memory_ptr_type
    }
}

pub struct MemoryRepresentation<'a> {
    m_context: &'a Context,
    m_builder: &'a Builder,
    m_module: &'a Module,
    m_memory: PointerValue,    
}

impl<'a> MemoryRepresentation<'a> {
    pub fn new(name: &str, context: &'a Context,
               builder: &'a Builder, module: &'a Module) -> MemoryRepresentation<'a> {
        let mem_type = MemoryRepresentationType::get_instance(context).get_type();
        let alloca_result = builder.build_alloca(mem_type, name);
        builder.build_store(alloca_result, mem_type.const_zero());

        MemoryRepresentation {
            m_context: context,
            m_builder: builder,
            m_module: module,
            m_memory: alloca_result
        }
    }

    pub fn get_memory_representation_type(&self) -> StructType {
        MemoryRepresentationType::get_instance(self.m_context).get_type()
    }
}

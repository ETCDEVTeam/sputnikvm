#![allow(dead_code)]

use std::ffi::CString;
use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::PointerValue;
use inkwell::AddressSpace;
use evmjit::BasicTypeEnumCompare;

#[derive(Debug, Singleton)]
/// Internal representation of EVM linear memory
pub struct MemoryRepresentationType {
    /// Type representing an EVM's memory segment
    memory_type: StructType,
    /// Type representing a pointer to an EVM memory
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
    /// Returns the LLVM type of an EVM memory segment.
    pub fn get_type(&self) -> StructType {
        self.memory_type
    }

    /// Returns the LLVM type of an EVM memory segment pointer.
    pub fn get_ptr_type(&self) -> PointerType {
        self.memory_ptr_type
    }
    
    /// Validates the properties of an EVM linear memory type.
    pub fn is_mem_representation_type(a_struct: &StructType) -> bool {
        if !a_struct.is_sized() {
            return false;
        }

        if a_struct.count_fields() != 3 {
            return false;
        }

        if a_struct.is_packed() {
            return false;
        }

        if a_struct.is_opaque() {
            return false;
        }

        if a_struct.get_name() != Some(&*CString::new("LinearMemory").unwrap()) {
            return false;
        }

        let field1 = a_struct.get_field_type_at_index(0).unwrap();
        if !field1.is_int256() {
            return false;
        }

        let field2 = a_struct.get_field_type_at_index(1).unwrap();
        if !field2.is_int64() {
            return false;
        }

        let field3 = a_struct.get_field_type_at_index(2).unwrap();
        if !field3.is_int64() {
            return false;
        }

        true

    }
}

/// An instance of an EVM linear memory.
pub struct MemoryRepresentation<'a> {
    m_context: &'a Context,
    m_builder: &'a Builder,
    m_module: &'a Module,
    m_memory: PointerValue,    
}

impl<'a> MemoryRepresentation<'a> {
    /// Constructs an EVM memory instance from a pre-allocated buffer.
    pub fn new_with_mem(allocated_memory: PointerValue, context: &'a Context,
                        builder: &'a Builder, module: &'a Module) -> MemoryRepresentation<'a> {
        let mem_type = MemoryRepresentationType::get_instance(context).get_type();
        builder.build_store(allocated_memory, mem_type.const_zero());

        MemoryRepresentation {
            m_context: context,
            m_builder: builder,
            m_module: module,
            m_memory: allocated_memory
        }

    }

    /// Constructs a named linear memory, handling allocation of memory space.
    pub fn new_with_name(name: &str, context: &'a Context,
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
    
    /// Returns the internal memory type representation.
    pub fn get_memory_representation_type(&self) -> StructType {
        MemoryRepresentationType::get_instance(self.m_context).get_type()
    }
}

#[test]
fn test_memory_representation_type() {
    let context = Context::create();
    let mem_type_singleton = MemoryRepresentationType::get_instance(&context);
    let mem_struct = mem_type_singleton.get_type();

    assert!(MemoryRepresentationType::is_mem_representation_type (&mem_struct));

    let mem_struct_ptr = mem_type_singleton.get_ptr_type();
    assert!(mem_struct_ptr.get_element_type().is_struct_type());
    assert!(MemoryRepresentationType::is_mem_representation_type (mem_struct_ptr.get_element_type().as_struct_type()));

    let evm_word_t = context.custom_width_int_type(256);
    let size_t = context.i64_type();

    let fields = [evm_word_t.into(),
                  size_t.into(),
                  context.i32_type().into()];

    let mem_struct2 = context.opaque_struct_type("LinearMemory");
    mem_struct2.set_body(&fields, false);

    assert!(!MemoryRepresentationType::is_mem_representation_type (&mem_struct2));
}

//#[test]

//fn test_memory_representation() {
//    let context = Context::create();
//    let module = context.create_module("test_module");
//    let builder = context.create_builder();
//
//    MemoryRepresentation::new(&context, &builder, &module);
//}

#![allow(dead_code)]

use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::BasicValueEnum;
use inkwell::values::PointerValue;
use evmjit::compiler::evmtypes::EvmTypes;
use evmjit::compiler::stack::EVM_MAX_STACK_SIZE;
use inkwell::module::Linkage::*;
use singletonum::Singleton;

pub struct StackAllocator {
    stack_base : BasicValueEnum,
    stack_size_ptr : PointerValue,
}

impl StackAllocator {
    pub fn new(context: & Context, builder: &Builder, module: &Module) -> StackAllocator {
        let types_instance = EvmTypes::get_instance(context);
        let malloc_fn_type = types_instance.get_word_ptr_type().fn_type(&[types_instance.get_size_type().into()], false);

        let malloc_func = module.add_function ("malloc", malloc_fn_type, Some(External));

        // TODO add Nounwind (i.e. function does not throw) and no alias attributes to function
        // Attribute::get_named_enum_kind_id("noalias"), return a u32 id number
        
        let malloc_size = (types_instance.get_word_type().get_bit_width() / 8) * EVM_MAX_STACK_SIZE;
        let malloc_size_ir_value = context.i64_type().const_int (malloc_size as u64, false);
        let base = builder.build_call (malloc_func, &[malloc_size_ir_value.into()], "stack_base");

        // m_stackSize = m_builder.CreateAlloca(Type::Size, nullptr, "stack.size");
	// m_builder.CreateStore(m_builder.getInt64(0), m_stackSize);

        let size_ptr = builder.build_alloca (types_instance.get_size_type(), "stack.size");
        builder.build_store (size_ptr, context.i64_type().const_zero());

        StackAllocator {
            stack_base: base.try_as_basic_value().left().unwrap(),
            stack_size_ptr: size_ptr
        }
    }

    pub fn get_stack_base_as_ir_value(&self) -> BasicValueEnum {
        self.stack_base
    }

    pub fn get_stack_size_as_ir_value(&self) -> PointerValue {
        self.stack_size_ptr
    }
}


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
use evmjit::LLVMAttributeFactory;

#[derive(Debug, Copy, Clone)]
pub struct StackAllocator {
    stack_base : BasicValueEnum,
    stack_size_ptr : PointerValue,
}

impl StackAllocator {
    pub fn new(context: & Context, builder: &Builder, module: &Module) -> StackAllocator {
        let types_instance = EvmTypes::get_instance(context);
        let malloc_fn_type = types_instance.get_word_ptr_type().fn_type(&[types_instance.get_size_type().into()], false);

        let malloc_func = module.add_function ("malloc", malloc_fn_type, Some(External));
        let attr_factory = LLVMAttributeFactory::get_instance(&context);

        malloc_func.add_attribute(0, *attr_factory.attr_nounwind());
        malloc_func.add_attribute(0, *attr_factory.attr_noalias());
        
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


#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use super::*;
    use inkwell::values::InstructionOpcode;
    use inkwell::attributes::Attribute;


    #[test]
    fn test_stack_allocator_new() {
        //use super::super::MainFuncCreator;
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();


        // Create dummy function

        let fn_type = context.void_type().fn_type(&[], false);
        let my_fn = module.add_function("my_fn", fn_type, Some(External));
        let entry_bb = context.append_basic_block(&my_fn, "entry");

        let attr_factory = LLVMAttributeFactory::get_instance(&context);

        builder.position_at_end(&entry_bb);
        StackAllocator::new(&context, &builder, &module);

        let malloc_func_optional = module.get_function("malloc");
        assert!(malloc_func_optional != None);

        let malloc_func = malloc_func_optional.unwrap();
        assert!(malloc_func.get_linkage() == External);

        let nounwind_attr = malloc_func.get_enum_attribute(0, Attribute::get_named_enum_kind_id("nounwind"));
        assert!(nounwind_attr != None);

        let noalias_attr = malloc_func.get_enum_attribute(0, Attribute::get_named_enum_kind_id("noalias"));
        assert!(noalias_attr != None);

        assert_eq!(nounwind_attr.unwrap(), *attr_factory.attr_nounwind());
        assert_eq!(noalias_attr.unwrap(), *attr_factory.attr_noalias());

        let entry_block_optional = my_fn.get_first_basic_block();
        assert!(entry_block_optional != None);
        let entry_block = entry_block_optional.unwrap();
        assert_eq!(*entry_block.get_name(), *CString::new("entry").unwrap());

        assert!(entry_block.get_first_instruction() != None);
        let first_insn = entry_block.get_first_instruction().unwrap();
        assert_eq!(first_insn.get_opcode(), InstructionOpcode::Call);

        assert!(first_insn.get_next_instruction() != None);
        let second_insn = first_insn.get_next_instruction().unwrap();
        assert_eq!(second_insn.get_opcode(), InstructionOpcode::Alloca);

        assert!(second_insn.get_next_instruction() != None);
        let third_insn = second_insn.get_next_instruction().unwrap();
        assert_eq!(third_insn.get_opcode(), InstructionOpcode::Store);

        assert!(third_insn.get_next_instruction() == None);
    }
}

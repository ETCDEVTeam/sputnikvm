#![allow(dead_code)]

pub mod env;
pub mod txctx;
pub mod stack_init;
pub mod rt_data_type;
pub mod rt_type;

use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::BasicValueEnum;
use inkwell::values::PointerValue;
use inkwell::values::FunctionValue;
use inkwell::basic_block::BasicBlock;
use inkwell::module::Linkage::*;
use singletonum::Singleton;
use self::rt_data_type::RuntimeDataType;
use self::rt_type::RuntimeType;
use self::rt_type::RuntimeTypeManager;
use self::txctx::TransactionContextManager;
use self::stack_init::StackAllocator;
use evmjit::compiler::evmtypes::EvmTypes;
use evmjit::compiler::evmconstants::EvmConstants;

pub struct MainFuncCreator {
    m_main_func: FunctionValue,
    m_jumptable_bb: BasicBlock,
    m_entry_bb: BasicBlock,
    m_stop_bb: BasicBlock,
    m_abort_bb: BasicBlock,
}

impl MainFuncCreator {
    pub fn new(name : &str, context: &Context, builder: &Builder, module: &Module) -> MainFuncCreator {

        let types_instance = EvmTypes::get_instance(context);
        let main_ret_type = types_instance.get_contract_return_type();

        let arg1 = RuntimeType::get_instance(context).get_ptr_type();
        
        let main_func_type = main_ret_type.fn_type(&[arg1.into()], false);
        let main_func = module.add_function (name, main_func_type, Some(External));
        main_func.get_first_param().unwrap().into_pointer_value().set_name("rt");

        let entry_bb = context.append_basic_block(&main_func, "Entry");
        let stop_bb = context.append_basic_block(&main_func, "Stop");
        let jumptable_bb = context.append_basic_block(&main_func, "JumpTable");
        let abort_bb = context.append_basic_block(&main_func, "Abort");

        builder.position_at_end(&jumptable_bb);
        let target = builder.build_phi(types_instance.get_word_type(), "target");
        builder.build_switch (*target.as_basic_value().as_int_value(), &abort_bb, &[]);
        builder.position_at_end(&entry_bb);
        
        MainFuncCreator {
            m_main_func: main_func,
            m_jumptable_bb: jumptable_bb,
            m_entry_bb: entry_bb,
            m_stop_bb: stop_bb,
            m_abort_bb: abort_bb,
        }
    }

    pub fn get_main_func(&self) -> FunctionValue {
        self.m_main_func
    }

    pub fn get_jumptable_bb(&self) -> &BasicBlock {
        &self.m_jumptable_bb
    }

    pub fn get_entry_bb(&self) -> &BasicBlock {
        &self.m_entry_bb
    }

    pub fn get_abort_bb(&self) -> &BasicBlock {
        &self.m_abort_bb
    }
}

struct GasPtrManager {
    m_gas_ptr: PointerValue
}

impl GasPtrManager {
    pub fn new(context: &Context, builder: &Builder, gas_value: BasicValueEnum) -> GasPtrManager {
        let types_instance = EvmTypes::get_instance(context);
        let gas_p = builder.build_alloca(types_instance.get_gas_type(), "gas.ptr");
        builder.build_store(gas_p, gas_value);

        GasPtrManager {
            m_gas_ptr: gas_p
        }
    }

    pub fn get_gas_ptr(&self) -> &PointerValue {
        &self.m_gas_ptr
    }
}

struct ReturnBufferManager<'a> {
    m_return_buf_data_ptr: PointerValue,
    m_return_buf_size_ptr: PointerValue,
    m_context: &'a Context,
    m_builder: &'a Builder,
}

impl<'a> ReturnBufferManager<'a> {
    pub fn new(context: &'a Context, builder: &'a Builder) -> ReturnBufferManager<'a> {
        let types_instance = EvmTypes::get_instance(context);
        let return_buf_data_p = builder.build_alloca(types_instance.get_byte_ptr_type(), "returndata.ptr");
        let return_buf_size_p = builder.build_alloca(types_instance.get_size_type(), "returndatasize.ptr");

        ReturnBufferManager {
            m_return_buf_data_ptr: return_buf_data_p,
            m_return_buf_size_ptr: return_buf_size_p,
            m_context: context,
            m_builder: builder
        }
    }

    pub fn get_return_buf_data_p(&self) -> &PointerValue {
        &self.m_return_buf_data_ptr
    }

    pub fn get_return_buf_size_p(&self) -> &PointerValue {
        &self.m_return_buf_size_ptr
    }

    pub fn reset_return_buf(self) {
        let const_factory = EvmConstants::get_instance(self.m_context);
        self.m_builder.build_store(self.m_return_buf_size_ptr, const_factory.get_i64_zero());
    }
}

pub struct RuntimeManager<'a> {
    m_context: &'a Context,
    m_builder: &'a Builder,
    m_module: &'a Module,
    m_txctx_manager:  TransactionContextManager<'a>,
    m_rt_type_manager: RuntimeTypeManager,
//    m_main_func_creator: MainFuncCreator, 
    m_stack_allocator: StackAllocator,
    m_gas_ptr_manager: GasPtrManager,
    m_return_buf_manager: ReturnBufferManager<'a>
}

impl<'a> RuntimeManager<'a> {
    pub fn new(context: &'a Context, builder: &'a Builder, module: &'a Module) -> RuntimeManager<'a> {
    //pub fn new(main_func_name: &str, context: &'a Context, builder: &'a Builder, module: &'a Module) -> RuntimeManager<'a> {

        // Generate outline of main function needed by 'RuntimeTypeManager
        //let main_func_creator = MainFuncCreator::new (&main_func_name, &context, &builder, &module);
        assert!(RuntimeManager::get_main_function_with_builder(builder, module) != None);

        // Generate IR for transaction context related items
        let txctx_manager = TransactionContextManager::new (&context, &builder, &module);

        // Generate IR for runtime type related items
        let rt_type_manager = RuntimeTypeManager::new (&context, &builder);

        let stack_allocator = StackAllocator::new (&context, &builder, &module);

        let gas_ptr_mgr = GasPtrManager::new(context, builder, rt_type_manager.get_gas());

        let return_buf_mgr = ReturnBufferManager::new(context, builder);

        RuntimeManager {
            m_context: context,
            m_builder: builder,
            m_module: module,
            m_txctx_manager: txctx_manager,
            m_rt_type_manager: rt_type_manager,
  //          m_main_func_creator: main_func_creator,
            m_stack_allocator: stack_allocator,
            m_gas_ptr_manager: gas_ptr_mgr,
            m_return_buf_manager: return_buf_mgr
        }
    }

    pub fn get_runtime_data_type(&self) -> StructType {
        RuntimeDataType::get_instance(self.m_context).get_type()
    }

    pub fn get_runtime_type(&self) -> StructType {
        RuntimeType::get_instance(self.m_context).get_type()
    }

    pub fn get_runtime_ptr_type(&self) -> PointerType {
        RuntimeType::get_instance(self.m_context).get_ptr_type()
    }

    pub fn get_runtime_ptr(&self) -> BasicValueEnum {
        // The parent of the first basic block is a function

        let bb = self.m_builder.get_insert_block();
        assert!(bb != None);
            
        let func = bb.unwrap().get_parent();
        assert!(func != None);
        let func_val = func.unwrap();

        // The first argument to a function is a pointer to the runtime
        assert!(func_val.count_params() > 0);

        let runtime_ptr = func_val.get_first_param().unwrap();
        assert_eq!(runtime_ptr.get_type().into_pointer_type(), self.get_runtime_ptr_type());

        runtime_ptr
    }

    pub fn get_gas_ptr(&self) -> &PointerValue {
        assert!(self.get_main_function() != None);
        self.m_gas_ptr_manager.get_gas_ptr()
    }

    pub fn get_gas(&self) -> BasicValueEnum {
        self.m_builder.build_load(*self.get_gas_ptr(), "gas")
    }

    pub fn get_return_buf_data_p(&self) -> &PointerValue {
        self.m_return_buf_manager.get_return_buf_data_p()
    }

    pub fn get_return_buf_size_p(&self) -> &PointerValue {
        self.m_return_buf_manager.get_return_buf_size_p()
    }

    pub fn reset_return_buf(self) {
        self.m_return_buf_manager.reset_return_buf()
    }

    fn get_main_function_with_builder(builder: & Builder, module: & Module) -> Option<FunctionValue> {
        // The parent of the first basic block is a function

        let bb = builder.get_insert_block();
        assert!(bb != None);

        let found_func = bb.unwrap().get_parent();
        assert!(found_func != None);
        let found_func_val = found_func.unwrap();

        // The main function (by convention) is the first one in the module
        let main_func = module.get_first_function();
        assert!(main_func != None);

        if found_func_val == main_func.unwrap() {
            found_func
        }
        else {
            None
        }
    }

    pub fn get_main_function(&self) -> Option<FunctionValue> {
        // The parent of the first basic block is a function

        let bb = self.m_builder.get_insert_block();
        assert!(bb != None);
            
        let found_func = bb.unwrap().get_parent();
        assert!(found_func != None);
        let found_func_val = found_func.unwrap();

        // The main function (by convention) is the first one in the module
        let main_func = self.m_module.get_first_function();
        assert!(main_func != None);

        if found_func_val == main_func.unwrap() {
            found_func
        }
        else {
            None
        }
    }
    
}

#[cfg(test)]
mod tests {
    use std::ffi::CString;
    use super::*;
    use inkwell::values::InstructionOpcode;

    #[test]
    fn test_runtime_manager() {
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Generate outline of main function needed by 'RuntimeTypeManager
        MainFuncCreator::new ("main", &context, &builder, &module);

        //let manager = RuntimeManager::new("main", &context, &builder, &module);
        let manager = RuntimeManager::new(&context, &builder, &module);


        assert!(RuntimeDataType::is_rt_data_type(&manager.get_runtime_data_type()));
        assert!(RuntimeType::is_runtime_type(&manager.get_runtime_type()));

        let rt_ptr = manager.get_runtime_ptr_type();
        assert!(rt_ptr.get_element_type().is_struct_type());
        assert!(RuntimeType::is_runtime_type(rt_ptr.get_element_type().as_struct_type()));
    }

    #[test]
    fn test_gas_ptr_manager() {
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Generate outline of main function needed by 'RuntimeTypeManager
        MainFuncCreator::new ("main", &context, &builder, &module);

        // Generate IR for runtime type related items
        let rt_type_manager = RuntimeTypeManager::new (&context, &builder);

        // Create dummy function

        let fn_type = context.void_type().fn_type(&[], false);
        let my_fn = module.add_function("my_fn", fn_type, Some(External));
        let entry_bb = context.append_basic_block(&my_fn, "entry");
        builder.position_at_end(&entry_bb);

        GasPtrManager::new(&context, &builder, rt_type_manager.get_gas());

        let entry_block_optional = my_fn.get_first_basic_block();
        assert!(entry_block_optional != None);
        let entry_block = entry_block_optional.unwrap();
        assert_eq!(*entry_block.get_name(), *CString::new("entry").unwrap());

        assert!(entry_block.get_first_instruction() != None);
        let first_insn = entry_block.get_first_instruction().unwrap();
        assert_eq!(first_insn.get_opcode(), InstructionOpcode::Alloca);

        assert!(first_insn.get_next_instruction() != None);
        let second_insn = first_insn.get_next_instruction().unwrap();
        assert_eq!(second_insn.get_opcode(), InstructionOpcode::Store);

        assert!(second_insn.get_next_instruction() == None);
    }


    #[test]
    fn test_return_buffer_manager() {
        let context = Context::create();
        let module = context.create_module("my_module");
        let builder = context.create_builder();

        // Create dummy function

        let fn_type = context.void_type().fn_type(&[], false);
        let my_fn = module.add_function("my_fn", fn_type, Some(External));
        let entry_bb = context.append_basic_block(&my_fn, "entry");
        builder.position_at_end(&entry_bb);

        ReturnBufferManager::new(&context, &builder);
        let entry_block_optional = my_fn.get_first_basic_block();
        assert!(entry_block_optional != None);
        let entry_block = entry_block_optional.unwrap();
        assert_eq!(*entry_block.get_name(), *CString::new("entry").unwrap());

        assert!(entry_block.get_first_instruction() != None);
        let first_insn = entry_block.get_first_instruction().unwrap();
        assert_eq!(first_insn.get_opcode(), InstructionOpcode::Alloca);

        assert!(first_insn.get_next_instruction() != None);
        let second_insn = first_insn.get_next_instruction().unwrap();
        assert_eq!(second_insn.get_opcode(), InstructionOpcode::Alloca);

        assert!(second_insn.get_next_instruction() == None);
    }


}


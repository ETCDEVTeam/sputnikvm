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

pub struct RuntimeManager<'a> {
    m_context: &'a Context,
    m_builder: &'a Builder,
    m_module: &'a Module,
    m_txctx_manager:  TransactionContextManager<'a>,
    m_rt_type_manager: RuntimeTypeManager,
    m_main_func_creator: MainFuncCreator, 
    m_stack_allocator: StackAllocator
}

impl<'a> RuntimeManager<'a> {
    pub fn new(main_func_name: &str, context: &'a Context, builder: &'a Builder, module: &'a Module) -> RuntimeManager<'a> {

        // Generate outline of main function needed by 'RuntimeTypeManager
        let main_func_creator = MainFuncCreator::new (&main_func_name, &context, &builder, &module);

        // Generate IR for transaction context related items
        let txctx_manager = TransactionContextManager::new (&context, &builder, &module);

        // Generate IR for runtime type related items
        let rt_type_manager = RuntimeTypeManager::new (&context, &builder);

        let stack_allocator = StackAllocator::new (&context, &builder, &module);
        
        RuntimeManager {
            m_context : context,
            m_builder : builder,
            m_module : module,
            m_txctx_manager: txctx_manager,
            m_rt_type_manager: rt_type_manager,
            m_main_func_creator: main_func_creator,
            m_stack_allocator: stack_allocator
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


#[test]


fn test_runtime_data_manager() {
    let context = Context::create();
    let module = context.create_module("my_module");
    let builder = context.create_builder();

    let manager = RuntimeManager::new("main", &context, &builder, &module);


    let rt_data_type_singleton = RuntimeDataType::get_instance(&context);
    let rt_data_struct = rt_data_type_singleton.get_type();

    assert_eq!(manager.get_runtime_data_type().count_fields(), rt_data_struct.count_fields());
    assert_eq!(manager.get_runtime_data_type().get_name(), rt_data_struct.get_name());
    assert_eq!(manager.get_runtime_data_type().get_field_types(), rt_data_struct.get_field_types());
    
    let rt_type_singleton = RuntimeType::get_instance(&context);
    let rt_struct = rt_type_singleton.get_type();

    assert_eq!(manager.get_runtime_type().count_fields(), rt_struct.count_fields());
    assert_eq!(manager.get_runtime_type().get_name(), rt_struct.get_name());
    assert_eq!(manager.get_runtime_type().get_field_types(), rt_struct.get_field_types());
}



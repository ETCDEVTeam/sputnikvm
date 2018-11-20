#![allow(dead_code)]

#[cfg(test)]
use std::ffi::CString;
use singletonum::{Singleton, SingletonInit};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::types::StructType;
use inkwell::types::PointerType;
use inkwell::values::PointerValue;
use inkwell::values::BasicValueEnum;
use inkwell::values::FunctionValue;
use inkwell::module::Linkage::*;
use inkwell::AddressSpace;
use evmjit::compiler::runtime::env::EnvDataType;
use evmjit::compiler::evmtypes::EvmTypes;
use llvm_sys::LLVMCallConv::*;

#[derive(PartialEq)]
pub enum TransactionContextTypeFields {
    GasPrice,
    Origin,
    CoinBase,
    Number,
    TimeStamp,
    GasLimit,
    Difficulty
}

trait TransactionContextTypeFieldToIndex {
    fn to_index(&self) -> usize;
}

impl TransactionContextTypeFieldToIndex for TransactionContextTypeFields {
    fn to_index(&self) -> usize {
        match self {
            TransactionContextTypeFields::GasPrice => 0,
            TransactionContextTypeFields::Origin => 1,
            TransactionContextTypeFields::CoinBase => 2,
            TransactionContextTypeFields::Number => 3,
            TransactionContextTypeFields::TimeStamp => 4,
            TransactionContextTypeFields::GasLimit => 5,
            TransactionContextTypeFields::Difficulty => 6,
        }
    }
}

#[derive(Debug, Singleton)]

pub struct TransactionContextType {
    txctx_type: StructType,
    txctx_ptr_type: PointerType,
}

unsafe impl Sync for TransactionContextType {}
unsafe impl Send for TransactionContextType {}

impl SingletonInit for TransactionContextType {
    type Init = Context;
    fn init(context: &Context) -> Self {
        let i64_t = context.i64_type();
        let i256_t = context.custom_width_int_type(256);
        let i8_t = context.i8_type();
        let array_of_160_bytes_t = i8_t.array_type(20);
        
        let fields = [i256_t.into(),            // Transaction gas price
                      array_of_160_bytes_t.into(),   // Transaction origin account
                      array_of_160_bytes_t.into(),   // Miner of the block (Coinbase)
                      i64_t.into(),                  // Block number
                      i64_t.into(),                  // block timestamp
                      i64_t.into(),                  // Block gas limit
                      i256_t.into()];                // Block difficulity
        
        let tx_struct = context.opaque_struct_type("evm.txctx");
        tx_struct.set_body(&fields, false);

        TransactionContextType {
            txctx_type : tx_struct,
            txctx_ptr_type : tx_struct.ptr_type(AddressSpace::Generic)
        }
    }
}

impl TransactionContextType {
    pub fn get_type(&self) -> StructType {
        self.txctx_type
    }

    pub fn get_ptr_type(&self) -> PointerType {
        self.txctx_ptr_type
    }
    
    pub fn get_num_fields(&self) -> u32 {
        self.get_type().count_fields()
    }
    
}


pub struct TransactionContextManager<'a> {
    m_tx_ctx_loaded : PointerValue,
    m_tx_ctx : PointerValue,
    m_load_tx_ctx_fn : FunctionValue,
    m_builder: &'a Builder,
    m_context: &'a Context,
}


impl<'a> TransactionContextManager<'a> {
    pub fn new(context: &'a Context, builder: &'a Builder, module: &Module) -> TransactionContextManager<'a> {
        let bool_t = context.bool_type();
        let tx_loaded = builder.build_alloca(bool_t, "txctx.loaded");
        builder.build_store(tx_loaded, bool_t.const_int(0, false));

        let env_data_singleton = EnvDataType::get_instance(&context);
        let tx_ctx_singleton = TransactionContextType::get_instance(&context);
        
        let tx_ctx_alloca = builder.build_alloca(tx_ctx_singleton.get_type(), "txctx");

        let tx_ctx_fn_t = context.void_type().fn_type(&[tx_ctx_alloca.get_type().into(),
                                                        env_data_singleton.get_ptr_type().into()], false);
        let tx_ctx_fn = module.add_function ("evm.get_tx_context", tx_ctx_fn_t, Some(External));

        let load_tx_ctx_fn_t = context.void_type().fn_type(&[tx_loaded.get_type().into(),
                                                             tx_ctx_alloca.get_type().into(),
                                                             env_data_singleton.get_ptr_type().into()],
                                                           false);
        let load_tx_ctx_fn = module.add_function ("loadTxCtx", load_tx_ctx_fn_t, Some(Private));
        
        load_tx_ctx_fn.set_call_conventions(LLVMFastCallConv as u32);

        let check_bb = context.append_basic_block(&load_tx_ctx_fn, "Check");
        let load_bb = context.append_basic_block(&load_tx_ctx_fn, "Load");
        let exit_bb = context.append_basic_block(&load_tx_ctx_fn, "Exit");

        let flag = load_tx_ctx_fn.get_nth_param(0).unwrap();
        flag.into_pointer_value().set_name("flag");

        let tx_ctx = load_tx_ctx_fn.get_nth_param(1).unwrap();
        tx_ctx.into_pointer_value().set_name("txctx");
        
        let env = load_tx_ctx_fn.get_nth_param(2).unwrap();
        env.into_pointer_value().set_name("env");

        let temp_builder = context.create_builder();
        temp_builder.position_at_end(&check_bb);

        let flag_value = temp_builder.build_load(flag.into_pointer_value(), "");
        temp_builder.build_conditional_branch(flag_value.into_int_value(), &exit_bb, &load_bb);

        temp_builder.position_at_end(&load_bb);
        temp_builder.build_store (flag.into_pointer_value(), bool_t.const_int(1,false));
        temp_builder.build_call (tx_ctx_fn, &[tx_ctx.into(), env.into()], "");
        temp_builder.build_unconditional_branch(&exit_bb);

        temp_builder.position_at_end(&exit_bb);
        temp_builder.build_return(None);
        
        TransactionContextManager {
            m_tx_ctx_loaded : tx_loaded,
            m_tx_ctx : tx_ctx_alloca,
            m_load_tx_ctx_fn : load_tx_ctx_fn,
            m_builder : builder,
            m_context : context
        }
    }

    pub fn get_tx_ctx_type(&self) -> & TransactionContextType {
        TransactionContextType::get_instance(self.m_context)
    }
    
    pub fn gen_tx_ctx_item_ir(&self, field : TransactionContextTypeFields) -> BasicValueEnum {
        let call = self.m_builder.build_call (self.m_load_tx_ctx_fn, &[self.m_tx_ctx_loaded.into(),
                                                             self.m_tx_ctx.into()], "");
        call.set_call_convention(LLVMFastCallConv as u32);
        let index = field.to_index();

        unsafe {
            let mut ptr = self.m_builder.build_struct_gep(self.m_tx_ctx, index as u32, "");

            // Origin and Coinbase are decalred as arrays of 20 bytes (160 bits) to deal with alignment issues
            // Cast back to i160 pointer here
            
            if field ==  TransactionContextTypeFields::Origin || field == TransactionContextTypeFields::CoinBase {
                let types_instance = EvmTypes::get_instance(self.m_context);
                ptr = self.m_builder.build_pointer_cast (ptr, types_instance.get_address_ptr_type(), "");
            }
            
            self.m_builder.build_load(ptr, "")
        }
    }
}



#[test]
fn test_tx_ctx_type() {
    let context = Context::create();
    let tx_ctx_type_singleton = TransactionContextType::get_instance(&context);
    let tx_ctx = tx_ctx_type_singleton.get_type();
    assert!(!tx_ctx.is_packed());
    assert!(!tx_ctx.is_opaque());
    assert!(tx_ctx.is_sized());
    assert_eq!(tx_ctx.get_name(), Some(&*CString::new("evm.txctx").unwrap()));
    assert_eq!(tx_ctx.count_fields(), 7);

    let i64_t = context.i64_type();
    let array_of_160_bytes_t = context.i8_type().array_type(20);
    let evm_word_t = context.custom_width_int_type(256);
    
    assert_eq!(tx_ctx.get_field_types(), &[evm_word_t.into(), array_of_160_bytes_t.into(),
                                           array_of_160_bytes_t.into(), i64_t.into(),
                                           i64_t.into(), i64_t.into(),
                                           evm_word_t.into()]);
}

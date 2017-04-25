use utils::bigint::M256;
use utils::gas::Gas;
use utils::address::Address;

use super::{Memory, VectorMemory, Stack, VectorStack, PC, VectorPC, Result, Error};
use blockchain::{Block, FakeVectorBlock};
use transaction::{Transaction, VectorTransaction};

use std::borrow::BorrowMut;
use std::marker::PhantomData;

pub trait Machine {
    type P: PC;
    type M: Memory;
    type Sta: Stack;
    type T: Transaction;
    type B: Block;
    type Sub: Machine;

    fn pc(&self) -> &Self::P;
    fn pc_mut(&mut self) -> &mut Self::P;
    fn memory(&self) -> &Self::M;
    fn memory_mut(&mut self) -> &mut Self::M;
    fn stack(&self) -> &Self::Sta;
    fn stack_mut(&mut self) -> &mut Self::Sta;

    fn transaction(&self) -> &Self::T;
    fn block(&self) -> &Self::B;
    fn block_mut(&mut self) -> &mut Self::B;

    fn use_gas(&mut self, gas: Gas);
    fn used_gas(&self) -> Gas;
    fn return_values(&self) -> &[u8];
    fn set_return_values(&mut self, data: &[u8]);

    fn fork<F: FnOnce(&mut Self::Sub)>(&mut self, gas: Gas, from:
                                       Address, to: Address, value: M256,
                                       memory_in_start: M256,
                                       memory_in_len: M256,
                                       memory_out_start: M256,
                                       memory_out_len: M256, f: F);

    fn step(&mut self) -> Result<()> where Self: Sized {
        if self.pc().stopped() {
            return Err(Error::Stopped)
        }

        if !self.available_gas().is_valid() {
            return Err(Error::EmptyGas)
        }

        let opcode = self.pc_mut().peek_opcode()?;
        let before = opcode.gas_cost_before(self)?;
        opcode.run(self)?;
        let after = opcode.gas_cost_after(self);

        self.pc_mut().read_opcode().unwrap();
        self.use_gas(before);
        self.use_gas(after);

        Ok(())
    }

    fn fire(&mut self) -> Result<()> where Self: Sized {
        loop {
            let result = self.step();

            if result.is_err() {
                match result.err().unwrap() {
                    Error::Stopped => return Ok(()),
                    err => return Err(err),
                }
            }
        }
    }

    fn available_gas(&self) -> Gas {
        self.transaction().gas_limit() - self.used_gas()
    }
}

pub struct VectorMachine<B0, BR> {
    pc: VectorPC,
    memory: VectorMemory,
    stack: VectorStack,
    transaction: VectorTransaction,
    return_values: Vec<u8>,
    block: Option<BR>,
    used_gas: Gas,
    _block_marker: PhantomData<B0>,
}

impl<B0: Block, BR: AsRef<B0> + AsMut<B0>> VectorMachine<B0, BR> {
    pub fn new(code: &[u8], data: &[u8], gas_limit: Gas,
               transaction: VectorTransaction, block: BR) -> Self {
        VectorMachine {
            pc: VectorPC::new(code),
            memory: VectorMemory::new(),
            stack: VectorStack::new(),
            transaction: transaction,
            return_values: Vec::new(),
            block: Some(block),
            used_gas: Gas::zero(),
            _block_marker: PhantomData,
        }
    }
}

impl<B0: Block, BR: AsRef<B0> + AsMut<B0>> Machine for VectorMachine<B0, BR> {
    type P = VectorPC;
    type M = VectorMemory;
    type Sta = VectorStack;
    type T = VectorTransaction;
    type B = B0;
    type Sub = Self;

    fn return_values(&self) -> &[u8] {
        self.return_values.as_ref()
    }

    fn set_return_values(&mut self, val: &[u8]) {
        self.return_values = val.into();
    }

    fn use_gas(&mut self, gas: Gas) {
        self.used_gas += gas;
    }

    fn used_gas(&self) -> Gas {
        self.used_gas
    }

    fn pc(&self) -> &Self::P {
        &self.pc
    }

    fn pc_mut(&mut self) -> &mut Self::P {
        &mut self.pc
    }

    fn memory(&self) -> &Self::M {
        &self.memory
    }

    fn memory_mut(&mut self) -> &mut Self::M {
        &mut self.memory
    }

    fn stack(&self) -> &Self::Sta {
        &self.stack
    }

    fn stack_mut(&mut self) -> &mut Self::Sta {
        &mut self.stack
    }

    fn transaction(&self) -> &Self::T {
        &self.transaction
    }

    fn block(&self) -> &Self::B {
        self.block.as_ref().unwrap().as_ref()
    }

    fn block_mut(&mut self) -> &mut Self::B {
        self.block.as_mut().unwrap().as_mut()
    }

    fn fork<F: FnOnce(&mut Self::Sub)>(&mut self, gas: Gas, from: Address, to: Address,
                                       value: M256,
                                       memory_in_start: M256, memory_in_len: M256,
                                       memory_out_start: M256, memory_out_len: M256,
                                       f: F) {
        use std::mem::swap;

        let from = from;
        let mem_in_start: usize = memory_in_start.into();
        let mem_in_len: usize = memory_in_len.into();
        let mem_in_end: usize = mem_in_start + mem_in_len;
        let mem_in: Vec<u8> = self.memory().as_ref()[mem_in_start..mem_in_end].into();

        let mut submachine = Self {
            pc: VectorPC::new(if to == from { self.pc().code() }
                              else { self.block().account_code(to) }),
            memory: VectorMemory::new(),
            stack: VectorStack::new(),
            transaction: VectorTransaction::message_call(from, to, value, mem_in.as_ref(), gas),
            return_values: Vec::new(),
            block: None,
            used_gas: Gas::zero(),
            _block_marker: PhantomData,
        };

        // We swap the block into the sub-machine if necessary. The
        // current old block should never be referenced and will be
        // replaced back when the call finishes.
        swap(&mut self.block, &mut submachine.block);
        f(self);
        swap(&mut self.block, &mut submachine.block);

        let mem_out_start: usize = memory_out_start.into();
        let mem_out_len: usize = memory_out_len.into();
        let mem_out_end: usize = mem_out_start + mem_out_len;

        for i in 0..mem_out_end {
            self.memory_mut().write_raw(memory_out_start + i.into(), submachine.return_values[i]);
        }
    }
}

pub type FakeVectorMachine = VectorMachine<FakeVectorBlock, Box<FakeVectorBlock>>;

impl FakeVectorMachine {
    pub fn fake(code: &[u8], data: &[u8], gas_limit: Gas) -> FakeVectorMachine {
        VectorMachine::new(code, data, gas_limit,
                           VectorTransaction::message_call(Address::default(), Address::default(),
                                                           M256::zero(), data, gas_limit),
                           Box::new(FakeVectorBlock::new()))
    }
}

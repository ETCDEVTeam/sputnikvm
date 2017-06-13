//! Cost calculation logic

use utils::gas::Gas;
use utils::address::Address;
use utils::bigint::{M256, U256};

use std::cmp::max;
use vm::{Memory, Instruction};
use super::State;
use super::precompiled::is_precompiled;

const G_ZERO: usize = 0;
const G_BASE: usize = 2;
const G_VERYLOW: usize = 3;
const G_LOW: usize = 5;
const G_MID: usize = 8;
const G_HIGH: usize = 10;
const G_JUMPDEST: usize = 1;
const G_SSET: usize = 20000;
const G_SRESET: usize = 5000;
const R_SCLEAR: usize = 15000;
const R_SUICIDE: usize = 24000;
const G_CREATE: usize = 32000;
const G_CODEDEPOSIT: usize = 200;
const G_CALLVALUE: usize = 9000;
const G_CALLSTIPEND: usize = 2300;
const G_NEWACCOUNT: usize = 25000;
const G_EXP: usize = 10;
const G_MEMORY: usize = 3;
const G_LOG: usize = 375;
const G_LOGDATA: usize = 8;
const G_LOGTOPIC: usize = 375;
const G_SHA3: usize = 30;
const G_SHA3WORD: usize = 6;
const G_COPY: usize = 3;
const G_BLOCKHASH: usize = 20;

fn sstore_cost<M: Memory + Default>(machine: &State<M>) -> Gas {
    let index = machine.stack.peek(0).unwrap();
    let value = machine.stack.peek(1).unwrap();
    let address = machine.context.address;

    if value != M256::zero() && machine.account_state.storage(address).unwrap().read(index).unwrap() == M256::zero() {
        G_SSET.into()
    } else {
        G_SRESET.into()
    }
}

fn call_cost<M: Memory + Default>(machine: &State<M>, is_callcode: bool) -> Gas {
    extra_cost(machine, is_callcode)
}

fn extra_cost<M: Memory + Default>(machine: &State<M>, is_callcode: bool) -> Gas {
    Gas::from(machine.patch.gas_call) + xfer_cost(machine) + new_cost(machine, is_callcode)
}

fn xfer_cost<M: Memory + Default>(machine: &State<M>) -> Gas {
    let val = machine.stack.peek(2).unwrap();
    if val != M256::zero() {
        G_CALLVALUE.into()
    } else {
        Gas::zero()
    }
}

fn new_cost<M: Memory + Default>(machine: &State<M>, is_callcode: bool) -> Gas {
    let address: Address = machine.stack.peek(1).unwrap().into();
    if machine.account_state.balance(address).unwrap() == U256::zero() && machine.account_state.nonce(address).unwrap() == M256::zero() && machine.account_state.code(address).unwrap().len() == 0 && !is_precompiled(address) && !is_callcode {
        G_NEWACCOUNT.into()
    } else {
        Gas::zero()
    }
}

fn suicide_cost<M: Memory + Default>(machine: &State<M>) -> Gas {
    let address: Address = machine.stack.peek(0).unwrap().into();
    Gas::from(machine.patch.gas_suicide) + if address == Address::default() {
        Gas::from(G_NEWACCOUNT)
    } else {
        Gas::zero()
    }
}

fn memory_expand(current: Gas, from: Gas, len: Gas) -> Gas {
    if len == Gas::zero() {
        return current;
    }

    let rem = (from + len) % Gas::from(32u64);
    let new = if rem == Gas::zero() {
        (from + len) / Gas::from(32u64)
    } else {
        (from + len) / Gas::from(32u64) + Gas::from(1u64)
    };
    max(current, new)
}

/// Calculate code deposit cost for a ContractCreation transaction.
pub fn code_deposit_gas(len: usize) -> Gas {
    Gas::from(G_CODEDEPOSIT) * Gas::from(len)
}

/// Calculate the memory gas from the memory cost.
pub fn memory_gas(a: Gas) -> Gas {
    (Gas::from(G_MEMORY) * a + a * a / Gas::from(512u64)).into()
}

/// Calculate the memory cost. This is the same as the active memory
/// length in the Yellow Paper.
pub fn memory_cost<M: Memory + Default>(instruction: Instruction, state: &State<M>) -> Gas {
    let ref stack = state.stack;

    let current = state.memory_cost;
    let next = match instruction {
        Instruction::SHA3 | Instruction::RETURN | Instruction::LOG(_) => {
            let from: U256 = stack.peek(0).unwrap().into();
            let len: U256 = stack.peek(1).unwrap().into();
            memory_expand(current, Gas::from(from), Gas::from(len))
        },
        Instruction::CODECOPY | Instruction::CALLDATACOPY => {
            let from: U256 = stack.peek(0).unwrap().into();
            let len: U256 = stack.peek(2).unwrap().into();
            memory_expand(current, Gas::from(from), Gas::from(len))
        },
        Instruction::EXTCODECOPY => {
            let from: U256 = stack.peek(1).unwrap().into();
            let len: U256 = stack.peek(3).unwrap().into();
            memory_expand(current, Gas::from(from), Gas::from(len))
        },
        Instruction::MLOAD | Instruction::MSTORE => {
            let from: U256 = stack.peek(0).unwrap().into();
            memory_expand(current, Gas::from(from), Gas::from(32u64))
        },
        Instruction::MSTORE8 => {
            let from: U256 = stack.peek(0).unwrap().into();
            memory_expand(current, Gas::from(from), Gas::from(1u64))
        },
        Instruction::CREATE => {
            let from: U256 = stack.peek(1).unwrap().into();
            let len: U256 = stack.peek(2).unwrap().into();
            memory_expand(current, Gas::from(from), Gas::from(len))
        },
        Instruction::CALL => {
            let in_from: U256 = stack.peek(3).unwrap().into();
            let in_len: U256 = stack.peek(4).unwrap().into();
            let out_from: U256 = stack.peek(5).unwrap().into();
            let out_len: U256 = stack.peek(6).unwrap().into();
            memory_expand(memory_expand(current, Gas::from(in_from), Gas::from(in_len)),
                          Gas::from(out_from), Gas::from(out_len))
        },
        _ => {
            current
        }
    };
    next
}

/// Calculate the gas cost.
pub fn gas_cost<M: Memory + Default>(instruction: Instruction, state: &State<M>) -> Gas {
    match instruction {
        Instruction::CALL => call_cost(state, false),
        Instruction::CALLCODE => call_cost(state, true),
        Instruction::DELEGATECALL => unimplemented!(),
        Instruction::SUICIDE => suicide_cost(state),
        Instruction::SSTORE => sstore_cost(state),

        Instruction::SHA3 => {
            let len = state.stack.peek(1).unwrap();
            let wordd = Gas::from(len) / Gas::from(32u64);
            let wordr = Gas::from(len) % Gas::from(32u64);
            (Gas::from(G_SHA3) + Gas::from(G_SHA3WORD) * if wordr == Gas::zero() { wordd } else { wordd + Gas::from(1u64) }).into()
        },

        Instruction::LOG(v) => {
            let len = state.stack.peek(1).unwrap();
            (Gas::from(G_LOG) + Gas::from(G_LOGDATA) * Gas::from(len) + Gas::from(G_LOGTOPIC) * Gas::from(v)).into()
        },

        Instruction::EXTCODECOPY => {
            let len = state.stack.peek(3).unwrap();
            let wordd = Gas::from(len) / Gas::from(32u64);
            let wordr = Gas::from(len) % Gas::from(32u64);
            (Gas::from(state.patch.gas_extcode) + Gas::from(G_COPY) * if wordr == Gas::zero() { wordd } else { wordd + Gas::from(1u64) }).into()
        },

        Instruction::CALLDATACOPY | Instruction::CODECOPY => {
            let len = state.stack.peek(2).unwrap();
            let wordd = Gas::from(len) / Gas::from(32u64);
            let wordr = Gas::from(len) % Gas::from(32u64);
            (Gas::from(G_VERYLOW) + Gas::from(G_COPY) * if wordr == Gas::zero() { wordd } else { wordd + Gas::from(1u64) }).into()
        },

        Instruction::EXP => {
            if state.stack.peek(1).unwrap() == M256::zero() {
                Gas::from(G_EXP)
            } else {
                Gas::from(G_EXP) + Gas::from(state.patch.gas_expbyte) * (Gas::from(1u64) + Gas::from(state.stack.peek(1).unwrap().log2floor()) / Gas::from(8u64))
            }
        }

        Instruction::CREATE => G_CREATE.into(),
        Instruction::JUMPDEST => G_JUMPDEST.into(),
        Instruction::SLOAD => state.patch.gas_sload.into(),

        // W_zero
        Instruction::STOP | Instruction::RETURN
            => G_ZERO.into(),

        // W_base
        Instruction::ADDRESS | Instruction::ORIGIN | Instruction::CALLER |
        Instruction::CALLVALUE | Instruction::CALLDATASIZE |
        Instruction::CODESIZE | Instruction::GASPRICE | Instruction::COINBASE |
        Instruction::TIMESTAMP | Instruction::NUMBER | Instruction::DIFFICULTY |
        Instruction::GASLIMIT | Instruction::POP | Instruction::PC |
        Instruction::MSIZE | Instruction::GAS
            => G_BASE.into(),

        // W_verylow
        Instruction::ADD | Instruction::SUB | Instruction::NOT | Instruction::LT |
        Instruction::GT | Instruction::SLT | Instruction::SGT | Instruction::EQ |
        Instruction::ISZERO | Instruction::AND | Instruction::OR | Instruction::XOR |
        Instruction::BYTE | Instruction::CALLDATALOAD | Instruction::MLOAD |
        Instruction::MSTORE | Instruction::MSTORE8 | Instruction::PUSH(_) |
        Instruction::DUP(_) | Instruction::SWAP(_)
            => G_VERYLOW.into(),

        // W_low
        Instruction::MUL | Instruction::DIV | Instruction::SDIV | Instruction::MOD |
        Instruction::SMOD | Instruction::SIGNEXTEND
            => G_LOW.into(),

        // W_mid
        Instruction::ADDMOD | Instruction::MULMOD | Instruction::JUMP
            => G_MID.into(),

        // W_high
        Instruction::JUMPI => G_HIGH.into(),

        // W_extcode
        Instruction::EXTCODESIZE => state.patch.gas_extcode.into(),
        Instruction::BALANCE => state.patch.gas_balance.into(),
        Instruction::BLOCKHASH => G_BLOCKHASH.into(),
    }
}

/// Raise gas stipend for CALL and CALLCODE instruction.
pub fn gas_stipend<M: Memory + Default>(instruction: Instruction, state: &State<M>) -> Gas {
    match instruction {
        Instruction::CALL | Instruction::CALLCODE => {
            let value = state.stack.peek(2).unwrap();

            if value != M256::zero() {
                G_CALLSTIPEND.into()
            } else {
                Gas::zero()
            }
        },
        _ => Gas::zero(),
    }
}

/// Calculate the refunded gas.
pub fn gas_refund<M: Memory + Default>(instruction: Instruction, state: &State<M>) -> Gas {
    match instruction {
        Instruction::SSTORE => {
            let index = state.stack.peek(0).unwrap();
            let value = state.stack.peek(1).unwrap();
            let address = state.context.address;

            if value == M256::zero() && state.account_state.storage(address).unwrap().read(index).unwrap() != M256::zero() {
                Gas::from(R_SCLEAR)
            } else {
                Gas::zero()
            }
        },
        Instruction::SUICIDE => {
            let address: Address = state.stack.peek(0).unwrap().into();
            if state.account_state.is_removed(address) {
                Gas::zero()
            } else {
                Gas::from(R_SUICIDE)
            }
        },
        _ => Gas::zero()
    }
}

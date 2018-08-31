//! Transaction related functionality.

#[cfg(not(feature = "std"))]
use alloc::Vec;

#[cfg(not(feature = "std"))] use alloc::rc::Rc;
#[cfg(feature = "std")] use std::rc::Rc;

#[cfg(feature = "std")] use std::collections::{HashSet as Set, hash_map as map};
#[cfg(feature = "std")] use std::cmp::min;
#[cfg(feature = "std")] use std::ops::Deref;
#[cfg(not(feature = "std"))] use alloc::{BTreeSet as Set, btree_map as map};
#[cfg(not(feature = "std"))] use core::cmp::min;
#[cfg(not(feature = "std"))] use core::ops::Deref;
use bigint::{U256, H256, Address, Gas};

use super::errors::{RequireError, CommitError, PreExecutionError};
use super::{State, Machine, Context, ContextVM, VM, AccountState,
            BlockhashState, Patch, HeaderParams, Memory, VMStatus,
            AccountCommitment, Log, AccountChange,
            Instruction, Opcode};

use block_core::TransactionAction;
#[cfg(feature = "std")]
use block::Transaction;

const G_TXDATAZERO: usize = 4;
const G_TXDATANONZERO: usize = 68;
const G_TRANSACTION: usize = 21000;

static SYSTEM_ADDRESS: [u8; 20] = [0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                   0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                   0xff, 0xff, 0xff, 0xff];

macro_rules! system_address {
    () => {
        Address::from(SYSTEM_ADDRESS.as_ref())
    }
}

#[derive(Debug, Clone)]
/// Represents an untrusted Ethereum transaction.
pub struct UntrustedTransaction {
    /// The caller. Must be attached with its commitment,
    pub caller: AccountCommitment,
    /// Transaction gas price.
    pub gas_price: Gas,
    /// Transaction gas limit.
    pub gas_limit: Gas,
    /// Action CALL/CREATE of the transaction.
    pub action: TransactionAction,
    /// Value sent with this transaction.
    pub value: U256,
    /// Transaction input.
    pub input: Rc<Vec<u8>>,
}

impl UntrustedTransaction {
    /// Convert to a valid transaction.
    pub fn to_valid<P: Patch>(&self) -> Result<ValidTransaction, PreExecutionError> {
        let valid = {
            let (nonce, balance, address) = match self.caller.clone() {
                AccountCommitment::Full { nonce, balance, address, .. } => {
                    (nonce, balance, address)
                },
                _ => return Err(PreExecutionError::InvalidCaller),
            };

            let gas_limit: U256 = self.gas_limit.into();
            let gas_price: U256 = self.gas_price.into();

            let (preclaimed_value, overflowed1) = gas_limit.overflowing_mul(gas_price);
            let (total, overflowed2) = preclaimed_value.overflowing_add(self.value);

            if overflowed1 || overflowed2 {
                return Err(PreExecutionError::InsufficientBalance);
            }

            if balance < total {
                return Err(PreExecutionError::InsufficientBalance);
            }

            ValidTransaction {
                caller: Some(address),
                gas_price: self.gas_price,
                gas_limit: self.gas_limit,
                action: self.action,
                value: self.value,
                input: self.input.clone(),
                nonce,
            }
        };

        if valid.gas_limit < valid.intrinsic_gas::<P>() {
            Err(PreExecutionError::InsufficientGasLimit)
        } else {
            Ok(valid)
        }
    }
}

#[derive(Debug, Clone)]
/// Represents an Ethereum transaction.
///
/// ## About SYSTEM transaction
///
/// SYSTEM transaction in Ethereum is something that cannot be
/// executed by the user, and is enforced by the blockchain rules. The
/// SYSTEM transaction does not have a caller. When executed in EVM,
/// however, the CALLER opcode would return
/// 0xffffffffffffffffffffffffffffffffffffffff. As a result, when
/// executing a message call or a contract creation, nonce are not
/// changed. A SYSTEM transaction must have gas_price set to zero.
/// Because the transaction reward is always zero, a SYSTEM
/// transaction will also not invoke creation of the beneficiary
/// address if it does not exist before.
pub struct ValidTransaction {
    /// Caller of this transaction. If caller is None, then this is a
    /// SYSTEM transaction.
    pub caller: Option<Address>,
    /// Gas price of this transaction.
    pub gas_price: Gas,
    /// Gas limit of this transaction.
    pub gas_limit: Gas,
    /// Transaction action.
    pub action: TransactionAction,
    /// Value of this transaction.
    pub value: U256,
    /// Data or init associated with this transaction.
    pub input: Rc<Vec<u8>>,
    /// Nonce of the transaction.
    pub nonce: U256,
}

#[cfg(feature = "std")]
impl ValidTransaction {
    /// Create a valid transaction from a block transaction. Caller is
    /// always Some.
    pub fn from_transaction<P: Patch>(
        transaction: &Transaction, account_state: &AccountState<P::Account>
    ) -> Result<Result<ValidTransaction, PreExecutionError>, RequireError> {
        let caller = match transaction.caller() {
            Ok(val) => val,
            Err(_) => return Ok(Err(PreExecutionError::InvalidCaller)),
        };

        let nonce = account_state.nonce(caller)?;
        if nonce != transaction.nonce {
            return Ok(Err(PreExecutionError::InvalidNonce));
        }

        let valid = ValidTransaction {
            caller: Some(caller),
            gas_price: transaction.gas_price,
            gas_limit: transaction.gas_limit,
            action: transaction.action,
            value: transaction.value,
            input: Rc::new(transaction.input.clone()),
            nonce,
        };

        if valid.gas_limit < valid.intrinsic_gas::<P>() {
            return Ok(Err(PreExecutionError::InsufficientGasLimit));
        }

        let balance = account_state.balance(caller)?;

        let gas_limit: U256 = valid.gas_limit.into();
        let gas_price: U256 = valid.gas_price.into();

        let (preclaimed_value, overflowed1) = gas_limit.overflowing_mul(gas_price);
        let (total, overflowed2) = preclaimed_value.overflowing_add(valid.value);
        if overflowed1 || overflowed2 {
            return Ok(Err(PreExecutionError::InsufficientBalance));
        }

        if balance < total {
            return Ok(Err(PreExecutionError::InsufficientBalance));
        }

        Ok(Ok(valid))
    }
}

impl ValidTransaction {
    /// To address of the transaction.
    pub fn address(&self) -> Address {
        self.action.address(self.caller.unwrap_or(system_address!()), self.nonce)
    }

    /// Intrinsic gas to be paid in prior to this transaction
    /// execution.
    pub fn intrinsic_gas<P: Patch>(&self) -> Gas {
        let mut gas = Gas::from(G_TRANSACTION);

        if self.action == TransactionAction::Create {
            gas = gas + P::gas_transaction_create();
        }

        for d in self.input.deref() {
            if *d == 0 {
                gas = gas + Gas::from(G_TXDATAZERO);
            } else {
                gas = gas + Gas::from(G_TXDATANONZERO);
            }
        }

        gas
    }

    /// Convert this transaction into a context. Note that this will
    /// change the account state.
    pub fn into_context<P: Patch>(
        self, upfront: Gas, origin: Option<Address>,
        account_state: &mut AccountState<P::Account>, is_code: bool,
        is_static: bool) -> Result<Context, RequireError> {
        let address = self.address();

        match self.action {
            TransactionAction::Call(_) => {
                if self.caller.is_some() {
                    account_state.require(self.caller.unwrap())?;
                }
                account_state.require_code(address)?;

                if self.caller.is_some() && !is_code {
                    let nonce = self.nonce;
                    account_state.set_nonce(self.caller.unwrap(), nonce + U256::from(1u64)).unwrap();
                }

                Ok(Context {
                    address,
                    caller: self.caller.unwrap_or(system_address!()),
                    data: self.input,
                    gas_price: self.gas_price,
                    value: self.value,
                    gas_limit: self.gas_limit - upfront,
                    code: account_state.code(address).unwrap(),
                    origin: origin.unwrap_or(self.caller.unwrap_or(system_address!())),
                    apprent_value: self.value,
                    is_system: self.caller.is_none(),
                    is_static
                })
            },
            TransactionAction::Create => {
                if self.caller.is_some() {
                    account_state.require(self.caller.unwrap())?;
                    let nonce = self.nonce;
                    account_state.set_nonce(self.caller.unwrap(), nonce + U256::from(1u64)).unwrap();
                }

                Ok(Context {
                    address,
                    caller: self.caller.unwrap_or(system_address!()),
                    gas_price: self.gas_price,
                    value: self.value,
                    gas_limit: self.gas_limit - upfront,
                    data: Rc::new(Vec::new()),
                    code: self.input,
                    origin: origin.unwrap_or(self.caller.unwrap_or(system_address!())),
                    apprent_value: self.value,
                    is_system: self.caller.is_none(),
                    is_static
                })
            },
        }
    }

    /// When the execution of a transaction begins, this preclaimed
    /// value is deducted from the account.
    pub fn preclaimed_value(&self) -> U256 {
        (self.gas_limit * self.gas_price).into()
    }
}

enum TransactionVMState<M, P: Patch> {
    Running {
        vm: ContextVM<M, P>,
        intrinsic_gas: Gas,
        preclaimed_value: U256,
        finalized: bool,
        code_deposit: bool,
        fresh_account_state: AccountState<P::Account>,
    },
    Constructing {
        transaction: ValidTransaction,
        block: HeaderParams,

        account_state: AccountState<P::Account>,
        blockhash_state: BlockhashState,
    },
}

/// A VM that executes using a transaction and block information.
pub struct TransactionVM<M, P: Patch>(TransactionVMState<M, P>);

impl<M: Memory + Default, P: Patch> TransactionVM<M, P> {
    /// Create a VM from an untrusted transaction. It can be any
    /// transaction and the VM will return an error if it has errors.
    pub fn new_untrusted(transaction: UntrustedTransaction, block: HeaderParams) -> Result<Self, PreExecutionError> {
        let valid = transaction.to_valid::<P>()?;
        let mut vm = TransactionVM(TransactionVMState::Constructing {
            transaction: valid,
            block,
            account_state: AccountState::default(),
            blockhash_state: BlockhashState::default(),
        });
        vm.commit_account(transaction.caller).unwrap();
        Ok(vm)
    }

    /// Create a new VM using the given transaction, block header and
    /// patch. This VM runs at the transaction level.
    pub fn new(transaction: ValidTransaction, block: HeaderParams) -> Self {
        TransactionVM(TransactionVMState::Constructing {
            transaction,
            block,
            account_state: AccountState::default(),
            blockhash_state: BlockhashState::default(),
        })
    }

    /// Create a new VM with the result of the previous VM. This is
    /// usually used by transaction for chaining them.
    pub fn with_previous(
        transaction: ValidTransaction, block: HeaderParams, vm: &TransactionVM<M, P>
    ) -> Self {
        TransactionVM(TransactionVMState::Constructing {
            transaction,
            block,
            account_state: match vm.0 {
                TransactionVMState::Constructing { ref account_state, .. } =>
                    account_state.clone(),
                TransactionVMState::Running { ref vm, .. } =>
                    vm.machines[0].state().account_state.clone(),
            },
            blockhash_state: match vm.0 {
                TransactionVMState::Constructing { ref blockhash_state, .. } =>
                    blockhash_state.clone(),
                TransactionVMState::Running { ref vm, .. } =>
                    vm.runtime.blockhash_state.clone(),
            },
        })
    }

    /// Returns the current state of the VM.
    pub fn current_state(&self) -> Option<&State<M, P>> {
        self.current_machine().map(|m| m.state())
    }

    /// Returns the current runtime machine.
    pub fn current_machine(&self) -> Option<&Machine<M, P>> {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => {
                Some(vm.current_machine())
            }
            TransactionVMState::Constructing { .. } => None,
        }
    }
}

impl<M: Memory + Default, P: Patch> VM for TransactionVM<M, P> {
    fn commit_account(&mut self, commitment: AccountCommitment) -> Result<(), CommitError> {
        match self.0 {
            TransactionVMState::Running { ref mut vm, .. } => vm.commit_account(commitment),
            TransactionVMState::Constructing { ref mut account_state, .. } => account_state.commit(commitment),
        }
    }

    fn commit_blockhash(&mut self, number: U256, hash: H256) -> Result<(), CommitError> {
        match self.0 {
            TransactionVMState::Running { ref mut vm, .. } => vm.commit_blockhash(number, hash),
            TransactionVMState::Constructing { ref mut blockhash_state, .. } => blockhash_state.commit(number, hash),
        }
    }

    fn status(&self) -> VMStatus {
        match self.0 {
            TransactionVMState::Running { ref vm, finalized, .. } => {
                if !finalized {
                    VMStatus::Running
                } else {
                    vm.status()
                }
            },
            TransactionVMState::Constructing { .. } => VMStatus::Running,
        }
    }

    fn peek(&self) -> Option<Instruction> {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.peek(),
            TransactionVMState::Constructing { .. } => None,
        }
    }

    fn peek_opcode(&self) -> Option<Opcode> {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.peek_opcode(),
            TransactionVMState::Constructing { .. } => None,
        }
    }

    fn step(&mut self) -> Result<(), RequireError> {
        let cgas: Gas;
        let ccontext: Context;
        let cblock: HeaderParams;
        let caccount_state: AccountState<P::Account>;
        let cblockhash_state: BlockhashState;
        let ccode_deposit: bool;
        let cpreclaimed_value: U256;

        let real_used_gas = self.used_gas();

        match self.0 {
            TransactionVMState::Running {
                ref mut vm,
                ref mut finalized,
                ref mut code_deposit,
                ref fresh_account_state,
                preclaimed_value,
                ..
            } => {
                match vm.status() {
                    VMStatus::Running => {
                        return vm.step();
                    },
                    VMStatus::ExitedNotSupported(_) => {
                        return Ok(());
                    },
                    _ => {
                        if *code_deposit {
                            vm.machines[0].code_deposit();
                            *code_deposit = false;
                            return Ok(());
                        }

                        if !*finalized {
                            vm.machines[0].finalize_transaction(vm.runtime.block.beneficiary,
                                                                real_used_gas, preclaimed_value,
                                                                fresh_account_state)?;
                            *finalized = true;
                            return Ok(());
                        }

                        return vm.step();
                    },
                }
            }
            TransactionVMState::Constructing {
                ref transaction, ref block,
                ref mut account_state, ref blockhash_state } => {

                let address = transaction.address();
                account_state.require(address)?;

                ccode_deposit = match transaction.action {
                    TransactionAction::Call(_) => false,
                    TransactionAction::Create => true,
                };
                cgas = transaction.intrinsic_gas::<P>();
                cpreclaimed_value = transaction.preclaimed_value();
                ccontext = transaction.clone().into_context::<P>(cgas, None, account_state, false, false)?;
                cblock = block.clone();
                caccount_state = account_state.clone();
                cblockhash_state = blockhash_state.clone();
            }
        }

        let account_state = caccount_state;
        let vm = ContextVM::with_init(
            ccontext, cblock,
            account_state.clone(),
            cblockhash_state,
            |vm| {
                if ccode_deposit {
                    vm.machines[0].initialize_create(cpreclaimed_value).unwrap();
                } else {
                    vm.machines[0].initialize_call(cpreclaimed_value).unwrap();
                }
            });

        self.0 = TransactionVMState::Running {
            fresh_account_state: account_state,
            vm,
            intrinsic_gas: cgas,
            finalized: false,
            code_deposit: ccode_deposit,
            preclaimed_value: cpreclaimed_value,
        };

        Ok(())
    }

    fn accounts(&self) -> map::Values<Address, AccountChange> {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.accounts(),
            TransactionVMState::Constructing { ref account_state, .. } => account_state.accounts(),
        }
    }

    fn used_addresses(&self) -> Set<Address> {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.used_addresses(),
            TransactionVMState::Constructing { ref account_state, .. } => account_state.used_addresses(),
        }
    }

    fn out(&self) -> &[u8] {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.out(),
            TransactionVMState::Constructing { .. } => &[],
        }
    }

    fn available_gas(&self) -> Gas {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.available_gas(),
            TransactionVMState::Constructing { ref transaction, .. } => transaction.gas_limit,
        }
    }

    fn refunded_gas(&self) -> Gas {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.refunded_gas(),
            TransactionVMState::Constructing { .. } => Gas::zero(),
        }
    }

    fn logs(&self) -> &[Log] {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.logs(),
            TransactionVMState::Constructing { .. } => &[],
        }
    }

    fn removed(&self) -> &[Address] {
        match self.0 {
            TransactionVMState::Running { ref vm, .. } => vm.removed(),
            TransactionVMState::Constructing { .. } => &[],
        }
    }

    fn used_gas(&self) -> Gas {
        match self.0 {
            TransactionVMState::Running { ref vm, intrinsic_gas, .. } => {
                let total_used = vm.machines[0].state().total_used_gas() + intrinsic_gas;
                let refund_cap = total_used / Gas::from(2u64);
                let refunded = min(refund_cap, vm.machines[0].state().refunded_gas);
                total_used - refunded
            }
            TransactionVMState::Constructing { .. } => Gas::zero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use ::*;
    use bigint::*;
    use hexutil::*;
    use block::TransactionAction;
    use std::str::FromStr;
    use std::rc::Rc;

    #[test]
    fn system_transaction() {
        let transaction = ValidTransaction {
            caller: None,
            gas_price: Gas::zero(),
            gas_limit: Gas::from_str("0xffffffffffffffff").unwrap(),
            action: TransactionAction::Call(Address::default()),
            value: U256::from_str("0xffffffffffffffff").unwrap(),
            input: Rc::new(Vec::new()),
            nonce: U256::zero(),
        };
        let mut vm = SeqTransactionVM::<EmbeddedPatch>::new(transaction, HeaderParams {
            beneficiary: Address::default(),
            timestamp: 0,
            number: U256::zero(),
            difficulty: U256::zero(),
            gas_limit: Gas::zero(),
        });
        vm.commit_account(AccountCommitment::Nonexist(Address::default())).unwrap();
        vm.fire().unwrap();

        let mut accounts: Vec<AccountChange> = Vec::new();
        for account in vm.accounts() {
            accounts.push(account.clone());
        }
        assert_eq!(accounts.len(), 1);
        match accounts[0] {
            AccountChange::Create {
                address, balance, ..
            } => {
                assert_eq!(address, Address::default());
                assert_eq!(balance, U256::from_str("0xffffffffffffffff").unwrap());
            },
            _ => panic!()
        }
    }

    #[test]
    fn system_transaction_non_zero_fee() {
        let transaction = ValidTransaction {
            caller: None,
            gas_price: Gas::one(),
            gas_limit: Gas::from_str("0xffffffffffffffff").unwrap(),
            action: TransactionAction::Call(Address::default()),
            value: U256::from_str("0xffffffffffffffff").unwrap(),
            input: Rc::new(Vec::new()),
            nonce: U256::zero(),
        };
        let mut vm = SeqTransactionVM::<EmbeddedPatch>::new(transaction, HeaderParams {
            beneficiary: Address::default(),
            timestamp: 0,
            number: U256::zero(),
            difficulty: U256::zero(),
            gas_limit: Gas::zero(),
        });
        vm.commit_account(AccountCommitment::Nonexist(Address::default())).unwrap();
        vm.fire().unwrap();

        let mut accounts: Vec<AccountChange> = Vec::new();
        for account in vm.accounts() {
            accounts.push(account.clone());
        }
        assert_eq!(accounts.len(), 1);
        match accounts[0] {
            AccountChange::Create {
                address, balance, ..
            } => {
                assert_eq!(address, Address::default());
                assert_eq!(balance, U256::from_str("0xffffffffffffffff").unwrap());
            },
            _ => panic!()
        }
    }

    #[test]
    fn eip140_spec_test() {
        let context = Context {
            address: Address::default(),
            caller: Address::default(),
            code: Rc::new(read_hex("6c726576657274656420646174616000557f726576657274206d657373616765000000000000000000000000000000000000600052600e6000fd").unwrap()),
            data: Rc::new(Vec::new()),
            gas_limit: Gas::from(100000usize),
            gas_price: Gas::from(0usize),
            origin: Address::default(),
            value: U256::zero(),
            apprent_value: U256::zero(),
            is_system: false,
            is_static: false,
        };

        let header = HeaderParams {
            beneficiary: Address::default(),
            timestamp: 0,
            number: U256::zero(),
            difficulty: U256::zero(),
            gas_limit: Gas::from(100000usize),
        };

        let mut vm = SeqContextVM::<EmbeddedByzantiumPatch>::new(context, header);
        vm.commit_account(AccountCommitment::Nonexist(Address::default())).unwrap();
        vm.fire().unwrap();

        assert_eq!(vm.used_gas(), Gas::from(20024usize));
        let out: Vec<u8> = vm.out().into();
        assert_eq!(out, read_hex("726576657274206d657373616765").unwrap());
        println!("accounts: {:?}", vm.accounts());
        assert_eq!(vm.accounts().len(), 0);
    }
}

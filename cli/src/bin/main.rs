#[macro_use]
extern crate clap;
extern crate sputnikvm;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate gethrpc;

use sputnikvm::{Gas, Address, U256, M256, read_hex};
use sputnikvm::vm::{BlockHeader, Context, SeqTransactionVM, Transaction, VM, Log, Patch,
                    AccountCommitment, Account, FRONTIER_PATCH, HOMESTEAD_PATCH,
                    EIP150_PATCH, EIP160_PATCH};
use sputnikvm::vm::errors::RequireError;
use gethrpc::{GethRPCClient, NormalGethRPCClient, RPCBlock};
use std::str::FromStr;
use std::fs::File;

mod serialize;

fn from_rpc_block(block: &RPCBlock) -> BlockHeader {
    BlockHeader {
        coinbase: Address::from_str(&block.miner).unwrap(),
        timestamp: M256::from_str(&block.timestamp).unwrap(),
        number: M256::from_str(&block.number).unwrap(),
        difficulty: M256::from_str(&block.difficulty).unwrap(),
        gas_limit: Gas::from_str(&block.gasLimit).unwrap(),
    }
}

fn handle_fire_without_rpc(vm: &mut SeqTransactionVM) {
    loop {
        match vm.fire() {
            Ok(()) => break,
            Err(RequireError::Account(address)) => {
                vm.commit_account(AccountCommitment::Nonexist(address)).unwrap();
            },
            Err(RequireError::AccountStorage(address, index)) => {
                vm.commit_account(AccountCommitment::Storage {
                    address: address,
                    index: index,
                    value: M256::zero(),
                }).unwrap();
            },
            Err(RequireError::AccountCode(address)) => {
                vm.commit_account(AccountCommitment::Code {
                    address: address,
                    code: Vec::new(),
                }).unwrap();
            },
            Err(RequireError::Blockhash(number)) => {
                vm.commit_blockhash(number, M256::zero());
            },
        }
    }
}

fn handle_fire_with_rpc<T: GethRPCClient>(client: &mut T, vm: &mut SeqTransactionVM, block_number: &str) {
    loop {
        match vm.fire() {
            Ok(()) => break,
            Err(RequireError::Account(address)) => {
                let nonce = M256::from_str(&client.get_transaction_count(&format!("0x{:x}", address),
                                                                         &block_number)).unwrap();
                let balance = U256::from_str(&client.get_balance(&format!("0x{:x}", address),
                                                                 &block_number)).unwrap();
                let code = read_hex(&client.get_code(&format!("0x{:x}", address),
                                                     &block_number)).unwrap();
                if !client.account_exist(&format!("0x{:x}", address), U256::from_str(&block_number).unwrap().as_usize()) {
                    vm.commit_account(AccountCommitment::Nonexist(address)).unwrap();
                } else {
                    vm.commit_account(AccountCommitment::Full {
                        nonce: nonce,
                        address: address,
                        balance: balance,
                        code: code,
                    }).unwrap();
                }
            },
            Err(RequireError::AccountStorage(address, index)) => {
                let value = M256::from_str(&client.get_storage_at(&format!("0x{:x}", address),
                                                                  &format!("0x{:x}", index),
                                                                  &block_number)).unwrap();
                vm.commit_account(AccountCommitment::Storage {
                    address: address,
                    index: index,
                    value: value,
                }).unwrap();
            },
            Err(RequireError::AccountCode(address)) => {
                let code = read_hex(&client.get_code(&format!("0x{:x}", address),
                                                     &block_number)).unwrap();
                vm.commit_account(AccountCommitment::Code {
                    address: address,
                    code: code,
                }).unwrap();
            },
            Err(RequireError::Blockhash(number)) => {
                let hash = M256::from_str(&client.get_block_by_number(&format!("0x{:x}", number))
                    .hash).unwrap();
                vm.commit_blockhash(number, hash);
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Config {
    create: bool,
    code: String,
    data: String,
    number: String,
    patch: String,
    gasLimit: String,
    gasPrice: String,
    caller: String,
    address: String,
    value: String
}

impl Default for Config {
    fn default() -> Self {
        Config {
            create: false,
            code: "".to_string(),
            data: "".to_string(),
            number: "0".to_string(),
            patch: "eip160".to_string(),
            gasLimit: "1000000000000000".to_string(),
            gasPrice: "0".to_string(),
            caller: "0x0000000000000000000000000000000000000000".to_string(),
            address: "0x0000000000000000000000000000000000000000".to_string(),
            value: "0".to_string(),
        }
    }
}

fn main() {
    let matches = clap_app!(sputnikvm =>
        (version: "0.1")
        (author: "Ethereum Classic Contributors")
        (about: "CLI tool for SputnikVM.")
        (@arg CREATE: --create "Execute a CreateContract transaction instead of message call.")
        (@arg CODE: --code +takes_value "Code to be executed.")
        (@arg RPC: --rpc +takes_value "Indicate this EVM should be run on an actual blockchain.")
        (@arg DATA: --data +takes_value "Data associated with this transaction.")
        (@arg BLOCK: --block +takes_value "Block number associated.")
        (@arg PATCH: --patch +takes_value "Patch to be used.")
        (@arg GAS_LIMIT: --gas_limit +takes_value "Gas limit.")
        (@arg GAS_PRICE: --gas_price +takes_value "Gas price, usually you will want this to be zero if no RPC endpoint is specified.")
        (@arg CALLER: --caller +takes_value "Caller of the transaction.")
        (@arg ADDRESS: --address +takes_value "Address of the transaction.")
        (@arg VALUE: --value +takes_value "Value of the transaction, usually you will want this to be zero if no RPC endpoint is specified.")
        (@arg FILE: --file +takes_value "Read config from a file.")
    ).get_matches();

    let config = if matches.value_of("FILE").is_some() {
        let file = File::open(matches.value_of("FILE").unwrap()).unwrap();
        serde_json::from_reader(file).unwrap()
    } else {
        Config::default()
    };

    let code = read_hex(matches.value_of("CODE").unwrap_or(&config.code)).unwrap();
    let data = read_hex(matches.value_of("DATA").unwrap_or(&config.data)).unwrap();
    let address = Address::from_str(matches.value_of("ADDRESS").unwrap_or(&config.address)).unwrap();
    let caller = Address::from_str(matches.value_of("CALLER").unwrap_or(&config.caller)).unwrap();
    let value = {
        let val = matches.value_of("VALUE").unwrap_or(&config.value);
        if val.starts_with("0x") {
            U256::from_str(val).unwrap()
        } else {
            let val: usize = val.parse().unwrap();
            U256::from(val)
        }
    };
    let gas_limit = {
        let val = matches.value_of("GAS_LIMIT").unwrap_or(&config.gasLimit);
        if val.starts_with("0x") {
            Gas::from_str(val).unwrap()
        } else {
            let val: usize = val.parse().unwrap();
            Gas::from(val)
        }
    };
    let gas_price = {
        let val = matches.value_of("GAS_PRICE").unwrap_or(&config.gasPrice);
        if val.starts_with("0x") {
            Gas::from_str(val).unwrap()
        } else {
            let val: usize = val.parse().unwrap();
            Gas::from(val)
        }
    };
    let block_number = {
        let val = matches.value_of("BLOCK").unwrap_or(&config.number);
        if val.starts_with("0x") {
            val.to_string()
        } else {
            let val: usize = val.parse().unwrap();
            format!("0x{:x}", val)
        }
    };

    let is_create = matches.is_present("CREATE") || config.create;
    let patch = match matches.value_of("PATCH").unwrap_or(&config.patch) {
        "frontier" => &FRONTIER_PATCH,
        "homestead" => &HOMESTEAD_PATCH,
        "eip150" => &EIP150_PATCH,
        "eip160" => &EIP160_PATCH,
        _ => panic!("Unsupported patch."),
    };

    let block = if matches.is_present("RPC") {
        let mut client = NormalGethRPCClient::new(matches.value_of("RPC").unwrap());
        from_rpc_block(&client.get_block_by_number(&block_number))
    } else {
        if gas_price > Gas::zero() || value > U256::zero() {
            panic!("Cannot continue as gas price or value is greater than zero but no account states is provided. You need to run this with a RPC endpoint.");
        }

        BlockHeader {
            coinbase: Address::default(),
            timestamp: M256::zero(),
            number: M256::from_str(&block_number).unwrap(),
            difficulty: M256::zero(),
            gas_limit: Gas::zero(),
        }
    };

    let transaction = if is_create {
        Transaction::ContractCreation {
            caller, value, gas_limit, gas_price,
            init: data,
        }
    } else {
        Transaction::MessageCall {
            caller, address, value, gas_limit, gas_price, data
        }
    };

    let mut vm = SeqTransactionVM::new(transaction, block, patch);
    if matches.is_present("RPC") {
        let mut client = NormalGethRPCClient::new(matches.value_of("RPC").unwrap());
        handle_fire_with_rpc(&mut client, &mut vm, &block_number);
    } else {
        handle_fire_without_rpc(&mut vm);
    };

    println!("{}", serialize::to_result(&vm));
}

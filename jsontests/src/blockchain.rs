use bigint::{Gas, M256, U256, H256, Address};
use hexutil::*;
use sputnikvm::{Log, Context,
                AccountChange, AccountCommitment,
                HeaderParams};

use serde_json::Value;
use self::ring::digest::{digest, SHA256};
use std::collections::HashMap;
use std::str::FromStr;
use std::rc::Rc;

pub struct JSONBlock {
    codes: HashMap<Address, Vec<u8>>,
    balances: HashMap<Address, U256>,
    storages: HashMap<Address, HashMap<U256, M256>>,
    nonces: HashMap<Address, U256>,

    beneficiary: Address,
    timestamp: u64,
    number: U256,
    difficulty: U256,
    gas_limit: Gas,

    logs: Vec<Log>,
}

static EMPTY: [u8; 0] = [];

impl JSONBlock {
    pub fn block_header(&self) -> HeaderParams {
        HeaderParams {
            beneficiary: self.beneficiary,
            timestamp: self.timestamp,
            number: self.number,
            difficulty: self.difficulty,
            gas_limit: self.gas_limit,
        }
    }

    pub fn request_account(&self, address: Address) -> AccountCommitment {
        let balance = self.balance(address);
        let code = self.account_code(address);
        let nonce = self.account_nonce(address);

        AccountCommitment::Full {
            address: address,
            balance: balance,
            code: Rc::new(code.into()),
            nonce: nonce
        }
    }

    pub fn request_account_storage(&self, address: Address, index: U256) -> AccountCommitment {
        let hashmap_default = HashMap::new();
        let storage = self.storages.get(&address).unwrap_or(&hashmap_default);
        let value = match storage.get(&index) {
            Some(val) => *val,
            None => M256::zero(),
        };

        AccountCommitment::Storage {
            address: address,
            index: index,
            value: value,
        }
    }

    pub fn request_account_code(&self, address: Address) -> AccountCommitment {
        let default = Vec::new();
        let code = self.codes.get(&address).unwrap_or(&default);

        AccountCommitment::Code {
            address: address,
            code: Rc::new(code.clone()),
        }
    }

    pub fn new_genesis_block() -> JSONBlock {
        JSONBlock::new("Genesis Block", vec![])
    }

    fn set_hash(&mut self) {
        let header = [
            &self.previous_hash[..],
            &self.data[..],
            &self.timestamp.to_string().as_bytes(),
        ].concat();
        self.hash = digest(&SHA256, &header[..]).as_ref().to_vec();
    }

    fn calculate_hash(&self) -> String {
        unimplemented!()
    }    
  
    pub fn apply_account(&mut self, account: AccountChange) {
        match account {
            AccountChange::Full {
                address,
                balance,
                changing_storage,
                code,
                nonce,
            } => {
                self.set_balance(address, balance);
                self.set_account_code(address, code.as_slice());
                if !self.storages.contains_key(&address) {
                    self.storages.insert(address, HashMap::new());
                }
                let changing_storage: HashMap<U256, M256> = changing_storage.into();
                for (key, value) in changing_storage {
                    self.storages.get_mut(&address).unwrap().insert(key, value);
                }
                self.set_account_nonce(address, nonce);
            },
            AccountChange::Create {
                address, balance, storage, code, nonce, ..
            } => {
                self.set_balance(address, balance);
                self.set_account_code(address, code.as_slice());
                self.storages.insert(address, storage.into());
                self.set_account_nonce(address, nonce);
            },
            AccountChange::Nonexist(address) => {
                self.set_balance(address, U256::zero());
                self.set_account_code(address, &[]);
                self.storages.insert(address, HashMap::new());
                self.set_account_nonce(address, U256::zero());
            },
            AccountChange::IncreaseBalance(address, topup) => {
                let balance = self.balance(address);
                self.set_balance(address, balance + topup);
            },
        }
    }

    pub fn apply_log(&mut self, log: Log) {
        self.logs.push(log);
    }

    pub fn coinbase(&self) -> Address {
        self.beneficiary
    }

    pub fn timestamp(&self) -> u64 {
        self.timestamp
    }

    pub fn number(&self) -> U256 {
        self.number
    }

    pub fn difficulty(&self) -> U256 {
        self.difficulty
    }

    pub fn gas_limit(&self) -> Gas {
        self.gas_limit
    }

    pub fn account_nonce(&self, address: Address) -> U256 {
        self.nonces.get(&address).map_or(U256::zero(), |s| (*s).into())
    }

    pub fn set_account_nonce(&mut self, address: Address, nonce: U256) {
        self.nonces.insert(address, nonce);
    }

    pub fn account_code(&self, address: Address) -> &[u8] {
        self.codes.get(&address).map_or(EMPTY.as_ref(), |s| s.as_ref())
    }

    pub fn set_account_code(&mut self, address: Address, code: &[u8]) {
        self.codes.insert(address, code.into());
    }

    pub fn balance(&self, address: Address) -> U256 {
        self.balances.get(&address).map_or(U256::zero(), |s| (*s).into())
    }

    pub fn set_balance(&mut self, address: Address, balance: U256) {
        self.balances.insert(address, balance);
    }

    pub fn account_storage(&self, address: Address, index: U256) -> M256 {
        match self.storages.get(&address) {
            None => M256::zero(),
            Some(ref ve) => {
                match ve.get(&index) {
                    Some(&v) => v,
                    None => M256::zero()
                }
            }
        }
    }

    pub fn set_account_storage(&mut self, address: Address, index: U256, val: M256) {
        if self.storages.get(&address).is_none() {
            self.storages.insert(address, HashMap::new());
        }

        let v = self.storages.get_mut(&address).unwrap();
        v.insert(index, val);
    }

    pub fn find_log(&self, address: Address, data: &[u8], topics: &[H256]) -> bool {
        for log in &self.logs {
            if log.address == address && log.data.as_slice() == data && log.topics.as_slice() == topics {
                return true;
            }
        }
        return false;
    }
}

pub fn create_block(v: &Value) -> JSONBlock {
    let mut block = {
        let env = &v["env"];

        let current_coinbase = env["currentCoinbase"].as_str().unwrap();
        let current_difficulty = env["currentDifficulty"].as_str().unwrap();
        let current_gas_limit = env["currentGasLimit"].as_str().unwrap();
        let current_number = env["currentNumber"].as_str().unwrap();
        let current_timestamp = env["currentTimestamp"].as_str().unwrap();

        JSONBlock {
            balances: HashMap::new(),
            storages: HashMap::new(),
            codes: HashMap::new(),
            nonces: HashMap::new(),

            beneficiary: Address::from_str(current_coinbase).unwrap(),
            difficulty: U256::from_str(current_difficulty).unwrap(),
            gas_limit: Gas::from_str(current_gas_limit).unwrap(),
            number: U256::from_str(current_number).unwrap(),
            timestamp: U256::from_str(current_timestamp).unwrap().into(),

            logs: Vec::new(),
        }
    };

    let ref pre_addresses = v["pre"];

    for (address, data) in pre_addresses.as_object().unwrap() {
        let address = Address::from_str(address.as_str()).unwrap();
        let balance = U256::from_str(data["balance"].as_str().unwrap()).unwrap();
        let code = read_hex(data["code"].as_str().unwrap()).unwrap();

        block.set_account_code(address, code.as_ref());
        block.set_balance(address, balance);

        let storage = data["storage"].as_object().unwrap();
        for (index, value) in storage {
            let index = U256::from_str(index.as_str()).unwrap();
            let value = M256::from_str(value.as_str().unwrap()).unwrap();
            block.set_account_storage(address, index, value);
        }
    }

    block
}

pub fn create_context(v: &Value) -> Context {
    let address = Address::from_str(v["exec"]["address"].as_str().unwrap()).unwrap();
    let caller = Address::from_str(v["exec"]["caller"].as_str().unwrap()).unwrap();
    let code = read_hex(v["exec"]["code"].as_str().unwrap()).unwrap();
    let data = read_hex(v["exec"]["data"].as_str().unwrap()).unwrap();
    let gas = Gas::from_str(v["exec"]["gas"].as_str().unwrap()).unwrap();
    let gas_price = Gas::from_str(v["exec"]["gasPrice"].as_str().unwrap()).unwrap();
    let origin = Address::from_str(v["exec"]["origin"].as_str().unwrap()).unwrap();
    let value = U256::from_str(v["exec"]["value"].as_str().unwrap()).unwrap();

    Context {
        address: address,
        caller: caller,
        code: Rc::new(code),
        data: Rc::new(data),
        gas_limit: gas,
        gas_price: gas_price,
        origin: origin,
        value: value,
        apprent_value: value,
        is_system: false,
        is_static: false,
    }
}

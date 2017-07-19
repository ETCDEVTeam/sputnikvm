use serde_json;
use sputnikvm::M256;
use sputnikvm::vm::{VM, SeqTransactionVM, Account};
use std::fmt::Write;
use std::collections::HashMap;

#[derive(Serialize)]
struct Result {
    status: String,
    out: String,
    usedGas: String,
    changingAccounts: Vec<AccountResult>,
    appendingLogs: Vec<LogResult>,
}

#[derive(Serialize)]
enum AccountResult {
    Full {
        nonce: String,
        address: String,
        balance: String,
        changingStorage: HashMap<String, String>,
        code: String,
    },
    IncreaseBalance {
        address: String,
        amount: String,
    },
    DecreaseBalance {
        address: String,
        amount: String,
    },
    New {
        nonce: String,
        address: String,
        balance: String,
        storage: HashMap<String, String>,
        code: String,
    },
    Remove {
        address: String,
    },
}

#[derive(Serialize)]
struct LogResult {
    address: String,
    data: String,
    topics: Vec<String>,
}

pub fn slice_to_hex(s: &[u8]) -> String {
    let mut r = String::new();
    write!(&mut r, "0x");
    for byte in s {
        write!(&mut r, "{:x}", byte);
    }
    r
}

pub fn to_result(vm: &SeqTransactionVM) -> String {
    let mut result = Result {
        status: format!("{:?}", vm.status()),
        out: slice_to_hex(vm.out()),
        usedGas: format!("0x{:x}", vm.real_used_gas()),
        changingAccounts: Vec::new(),
        appendingLogs: Vec::new(),
    };

    for account in vm.accounts() {
        match account {
            &Account::IncreaseBalance(address, amount) => {
                result.changingAccounts.push(AccountResult::IncreaseBalance {
                    address: format!("0x{:x}", address),
                    amount: format!("0x{:x}", amount),
                });
            },
            &Account::DecreaseBalance(address, amount) => {
                result.changingAccounts.push(AccountResult::DecreaseBalance {
                    address: format!("0x{:x}", address),
                    amount: format!("0x{:x}", amount),
                });
            },
            &Account::Full {
                nonce, address, balance, ref changing_storage, ref code
            } => {
                let changing_storage: HashMap<M256, M256> = changing_storage.clone().into();
                let mut changingStorage = HashMap::new();
                for (k, v) in changing_storage.iter() {
                    changingStorage.insert(format!("0x{:x}", k),
                                           format!("0x{:x}", v));
                }
                result.changingAccounts.push(AccountResult::Full {
                    nonce: format!("0x{:x}", nonce),
                    address: format!("0x{:x}", address),
                    balance: format!("0x{:x}", balance),
                    code: slice_to_hex(code),
                    changingStorage,
                });
            },
            &Account::Create {
                nonce, address, balance, ref storage, ref code, exists,
            } => {
                if exists {
                    let storage: HashMap<M256, M256> = storage.clone().into();
                    let mut storageResult = HashMap::new();
                    for (k, v) in storage.iter() {
                        storageResult.insert(format!("0x{:x}", k),
                                             format!("0x{:x}", v));
                    }
                    result.changingAccounts.push(AccountResult::New {
                        nonce: format!("0x{:x}", nonce),
                        address: format!("0x{:x}", address),
                        balance: format!("0x{:x}", balance),
                        code: slice_to_hex(code),
                        storage: storageResult,
                    });
                } else {
                    result.changingAccounts.push(AccountResult::Remove {
                        address: format!("0x{:x}", address),
                    });
                }
            },
        }
    }

    for log in vm.logs() {
        let mut topics = Vec::new();
        for topic in &log.topics {
            topics.push(format!("0x{:x}", topic));
        }
        result.appendingLogs.push(LogResult {
            address: format!("0x{:x}", log.address),
            data: slice_to_hex(&log.data),
            topics
        });
    }

    serde_json::to_string_pretty(&result).unwrap()
}

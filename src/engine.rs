use anyhow::{Context, Error, Result};
use std::collections::HashMap;
use std::io::BufWriter;
use std::io::Write;

#[derive(Debug, Clone)]
enum TxType {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
    Noop,
}

impl Default for TxType {
    fn default() -> Self {
        Self::Noop
    }
}

impl From<&str> for TxType {
    fn from(value: &str) -> Self {
        match value {
            "deposit" => Self::Deposit,
            "withdrawal" => Self::Withdrawal,
            "dispute" => Self::Dispute,
            "resolve" => Self::Resolve,
            "chargeback" => Self::Chargeback,
            _ => unreachable!("invalid Tx type"),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(crate) struct Tx {
    tx_type: TxType,
    tx_id: u32,
    client: u16,
    amount: Option<f64>,
}

impl Tx {
    pub(crate) fn from_str(v: &str) -> Result<Self> {
        let d: Vec<&str> = v
            .splitn(4, &[',', ';'])
            .filter_map(|chunk| Some(chunk.trim()))
            .collect();

        let tx_type = d
            .get(0)
            .ok_or_else(|| Error::msg("missing transaction type"))?
            .to_owned()
            .into();
        let client = d
            .get(1)
            .ok_or_else(|| Error::msg("missing client"))?
            .parse::<u16>()
            .context("could not parse client to u16")?;
        let tx_id = d
            .get(2)
            .ok_or_else(|| Error::msg("missing transaction"))?
            .parse::<u32>()
            .context("could not parse tx to u32")?;
        let amount = d
            .get(3)
            .and_then(|v| Some(v.parse::<f64>().unwrap_or(0.)));
        Ok(Self {
            tx_type,
            client,
            tx_id,
            amount,
        })
    }
}

#[derive(Debug, Clone, Default)]
struct Account {
    client: u16,
    available: f64,
    held: f64,
    total: f64,
    locked: bool,
}

impl Account {
    fn to_csv_line(&self) -> String {
        format!(
            "{},{},{},{},{}",
            self.client, self.available, self.held, self.total, self.locked
        )
    }
}

type ClientId = u16;
type TxId = u32;

pub(crate) struct TxEngine {
    accounts: HashMap<ClientId, Account>,
    txs: HashMap<TxId, Tx>,
    desputes: HashMap<TxId, Tx>,
}

impl TxEngine {
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
            txs: HashMap::default(),
            desputes: HashMap::new(),
        }
    }

    pub fn process_tx(&mut self, tx: Tx) {
        match tx.tx_type {
            TxType::Deposit | TxType::Withdrawal => {
                self.process_deposit_and_withdrawal(tx);
            }
            TxType::Dispute => {
                self.process_dispute(tx.tx_id);
            }
            TxType::Resolve => {
                self.process_resolve(tx.tx_id);
            }
            TxType::Chargeback => {
                self.process_chargeback(tx.tx_id);
            }
            _ => unreachable!("unidentified transaction type"),
        }
    }

    fn process_deposit_and_withdrawal(&mut self, tx: Tx) {
        let account = self.accounts.entry(tx.client).or_insert_with(|| Account {
            client: tx.client,
            ..Default::default()
        });

        if account.locked {
            return;
        }

        match tx.tx_type {
            TxType::Deposit => {
                if let Some(amount) = tx.amount {
                    account.available += amount;
                    account.total += amount;
                    self.txs.insert(tx.tx_id, tx);
                }
            }
            TxType::Withdrawal => {
                if let Some(amount) = tx.amount {
                    if account.available >= amount {
                        account.available -= amount;
                        account.total -= amount;
                    }
                    self.txs.insert(tx.tx_id, tx);
                }
            }
            _ => unreachable!(),
        }
    }
    fn process_dispute(&mut self, tx_id: TxId) {
        if let Some(tx) = self.txs.get(&tx_id) {
            if let Some(amount) = tx.amount {
                // we do know she/he has account;
                let account = self.accounts.get_mut(&tx.client).unwrap();
                account.available -= amount;
                account.held += amount;
                self.desputes.insert(tx_id, tx.clone());
            }
        }
    }
    fn process_resolve(&mut self, tx_id: TxId) {
        if let Some(tx) = self.txs.get(&tx_id) {
            if let Some(amount) = tx.amount {
                // we do know she/he has account;
                let account = self.accounts.get_mut(&tx.client).unwrap();
                account.available += amount;
                account.held -= amount;
                self.desputes.insert(tx_id, tx.clone());
            }
        }
    }
    fn process_chargeback(&mut self, tx_id: TxId) {
        if let Some(tx) = self.txs.get(&tx_id) {
            if let Some(amount) = tx.amount {
                // we do know she/he has account;
                let account = self.accounts.get_mut(&tx.client).unwrap();
                account.total -= amount;
                account.held -= amount;
                account.locked = true;
            }
        }
    }

    pub(crate) fn summarize_accounts(&self, w: impl Write) -> Result<()> {
        let mut writer = BufWriter::new(w);
        writeln!(writer, "{}", "client,available,held,total,locked")?;
        for client in self.accounts.values() {
            writeln!(writer, "{}", client.to_csv_line())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dispute_resolve_and_chargeback_flow() {
        let mut engine = TxEngine::new();

        engine.process_tx(Tx {
            tx_type: TxType::Deposit,
            client: 1,
            tx_id: 1,
            amount: Some(1000.0),
        });
        engine.process_tx(Tx {
            tx_type: TxType::Deposit,
            client: 1,
            tx_id: 2,
            amount: Some(500.0),
        });

        engine.process_tx(Tx {
            tx_type: TxType::Dispute,
            client: 1,
            tx_id: 1,
            amount: None,
        });

        {
            let account = engine.accounts.get(&1).unwrap();
            assert_eq!(account.available, 500.0); 
            assert_eq!(account.held, 1000.0); 
            assert_eq!(account.total, 1500.0);
            assert!(!account.locked);
        }

        engine.process_tx(Tx {
            tx_type: TxType::Resolve,
            client: 1,
            tx_id: 1,
            amount: None,
        });

        {
            let account = engine.accounts.get(&1).unwrap();
            assert_eq!(account.available, 1500.0); 
            assert_eq!(account.held, 0.0); 
            assert_eq!(account.total, 1500.0); 
            assert!(!account.locked);
        }

        engine.process_tx(Tx {
            tx_type: TxType::Dispute,
            client: 1,
            tx_id: 2,
            amount: None,
        });
        engine.process_tx(Tx {
            tx_type: TxType::Chargeback,
            client: 1,
            tx_id: 2,
            amount: None,
        });

        {
            let account = engine.accounts.get(&1).unwrap();
            assert_eq!(account.available, 1000.0);
            assert_eq!(account.held, 0.0); 
            assert_eq!(account.total, 1000.0); 
            assert!(account.locked); 
        }
    }
}

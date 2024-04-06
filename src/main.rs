mod engine;
mod csv_stream;
use anyhow::{Result, Context};
use engine::*;
use std::fs::File;
use std::io::BufRead;
use std::io::BufReader;
use std::io::StdoutLock;
use std::path::PathBuf;

fn reader_loop(file_path: &PathBuf, stdout: &mut StdoutLock) -> Result<()> {
    let f = File::open(file_path)?;
    let reader = BufReader::new(f);

    let mut tx_engine = TxEngine::new();

    for line in reader.lines().skip(1) {
        let line = line?;
        if line.is_empty() { continue; }

        let tx = Tx::from_str(&line).context(format!("could not convert {} to {}", "str", "Tx"))?;
        tx_engine.process_tx(tx);
    }
    tx_engine.summarize_accounts(stdout)?;
    Ok(())
}



#[tokio::main]
async fn main() -> Result<()> {
    let mut stdout = std::io::stdout().lock();
    let mut args = std::env::args().skip(1);
    match args.next() {
        Some(f_path) => {
            let file_path = PathBuf::from(f_path);
            reader_loop(&file_path, &mut stdout)?;
        }
        None => {
            csv_stream::handle_stream().await?;
        }
    }
    Ok(())
}

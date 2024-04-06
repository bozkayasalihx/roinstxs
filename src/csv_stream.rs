use crate::{Tx, TxEngine};
use anyhow::Result;
use std::io::Write;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::TcpListener;
use tokio::sync::Mutex;

const HOST: &str = "127.0.0.1:6969";

struct TestWriter;
impl Write for TestWriter {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        todo!();
    }
    fn flush(&mut self) -> std::io::Result<()> {
        todo!();
    }
}

unsafe impl Send for TestWriter {}

pub async fn handle_stream() -> Result<()> {
    let tx_engine = Arc::new(Mutex::new(TxEngine::new()));
    let listener = TcpListener::bind(HOST).await?;

    loop {
        let (socket, _) = listener.accept().await?;
        let tx_engine_clone = tx_engine.clone();

        tokio::spawn(async move {
            if let Err(err) = handle_connection(socket, tx_engine_clone).await {
                eprintln!("could not handle conn: {}", err);
            }
        });
    }
}

async fn handle_connection(
    socket: tokio::net::TcpStream,
    engine: Arc<Mutex<TxEngine>>,
) -> Result<()> {
    let reader = BufReader::new(socket);
    let mut lines = reader.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.is_empty() { continue; }

        let tx = match Tx::from_str(&line) {
            Ok(tx) => tx,
            Err(err) => {
                eprintln!("error processing trasnactions {}", err);
                continue;
            }
        };
        let mut engine = engine.lock().await;
        engine.process_tx(tx);
    }

    // NOTE: The destination for these summarized accounts is not specified.
    //       Any entity that implements the `Write` trait is acceptable as a destination.
    //       It could be a Kafka connector, a writer for SQL or NoSQL databases
    let engine = engine.lock().await;
    engine.summarize_accounts(TestWriter).unwrap();

    Ok(())
}

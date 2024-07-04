use std::net::SocketAddr;

use anyhow::Result;
use async_channel::bounded;
use localex::manager::PairingManager;

#[tokio::main]
async fn main() -> Result<()> {
    let pair_manager = PairingManager::new().await?;
    let (mut pair_sender, mut pair_listener) = pair_manager.split();
    let (on_pair_tx, on_pair_rx) = bounded::<SocketAddr>(3);

    // try to pairing on launch
    pair_sender.try_pairing().await?;

    let pairing_handle = tokio::spawn(async move {
        let handle = tokio::runtime::Handle::current();
        let _ = pair_listener.event_loop(Box::new(move |addr| {
            let _ = handle.block_on(on_pair_tx.send(addr));
        })).await;
    });

    let onpair_handle = tokio::spawn(async move {
        loop {
            if let Ok(addr) = on_pair_rx.recv().await {
                todo!()
            }
        }
    });

    tokio::select! {
        _ = pairing_handle => {},
        _ = onpair_handle => {},
    }

    Ok(())
}

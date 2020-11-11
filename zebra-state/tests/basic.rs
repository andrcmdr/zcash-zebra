use color_eyre::eyre::Report;
use once_cell::sync::Lazy;
use std::sync::Arc;
use tempdir::TempDir;
use zebra_chain::{block::Block, serialization::ZcashDeserialize};
use zebra_test::transcript::Transcript;

use zebra_state::*;

static ADD_BLOCK_TRANSCRIPT: Lazy<Vec<(RequestBlock, Response)>> = Lazy::new(|| {
    let block: Arc<_> =
        Block::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])
            .unwrap()
            .into();
    let hash = block.as_ref().into();
    let height = block.coinbase_height().unwrap();
    vec![
        (
            RequestBlock::AddBlock {
                block: block.clone(),
            },
            Response::Added { hash: hash, height: height },
        ),
        (RequestBlock::GetBlock { query: QueryType::ByHash(hash) }, Response::Block { block: block.clone() }),
        (RequestBlock::GetBlock { query: QueryType::ByHeight(height) }, Response::Block { block: block.clone() }),
    ]
});

static GET_TIP_TRANSCRIPT: Lazy<Vec<(RequestBlock, Response)>> = Lazy::new(|| {
    let block0: Arc<_> =
        Block::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_GENESIS_BYTES[..])
            .unwrap()
            .into();
    let block1: Arc<_> = Block::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_1_BYTES[..])
        .unwrap()
        .into();
    let hash0 = block0.as_ref().into();
    let height0 = block0.coinbase_height().unwrap();
    let hash1 = block1.as_ref().into();
    let height1 = block1.coinbase_height().unwrap();
    vec![
        // Insert higher block first, lower block second
        (
            RequestBlock::AddBlock { block: block1 },
            Response::Added { hash: hash1, height: height1 },
        ),
        (
            RequestBlock::AddBlock { block: block0 },
            Response::Added { hash: hash0, height: height0 },
        ),
        (RequestBlock::GetTip, Response::Tip { hash: hash1, height: height1 }),
    ]
});

#[tokio::test]
async fn check_transcripts() -> Result<(), Report> {
    zebra_test::init();

    for transcript_data in &[&ADD_BLOCK_TRANSCRIPT, &GET_TIP_TRANSCRIPT] {
        let service = in_memory::init();
        let transcript = Transcript::from(transcript_data.iter().cloned());
        transcript.check(service).await?;

        let storage_guard = TempDir::new("")?;
        let service = on_disk::init(Config {
            path: storage_guard.path().to_owned(),
        });
        let transcript = Transcript::from(transcript_data.iter().cloned());
        transcript.check(service).await?;
        // Delete the contents of the temp directory before going to the next case.
        std::mem::drop(storage_guard);
    }

    Ok(())
}

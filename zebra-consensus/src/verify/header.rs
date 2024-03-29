//! Block header verification and chain state updates for Zebra.
//!
//! Verification occurs in multiple stages:
//!   - getting block headers (disk- or network-bound)
//!   - context-free verification of signatures, proofs, and scripts (CPU-bound)
//!   - context-dependent verification of the chain state (awaits a verified parent block header)
//!
//! Verification is provided via a `tower::Service`, to support backpressure and batch
//! verification.

use chrono::{DateTime, Duration, Utc};
use futures_util::FutureExt;
use std::{
    error,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};
use tower::{buffer::Buffer, Service, ServiceExt};

use zebra_chain::{
    block::{BlockHeader, BlockHeaderHash},
    types::BlockHeight,
};

// use zebra_state::QueryType;

/// Check if `block_header_time` is less than or equal to
/// 2 hours in the future, according to the node's local clock (`now`).
///
/// This is a non-deterministic rule, as clocks vary over time, and
/// between different nodes.
///
/// "In addition, a full validator MUST NOT accept blocks with nTime
/// more than two hours in the future according to its clock. This
/// is not strictly a consensus rule because it is nondeterministic,
/// and clock time varies between nodes. Also note that a block that
/// is rejected by this rule at a given point in time may later be
/// accepted."[S 7.5][7.5]
///
/// [7.5]: https://zips.z.cash/protocol/protocol.pdf#blockheader
pub(crate) fn node_time_check(
    block_header_time: DateTime<Utc>,
    now: DateTime<Utc>,
) -> Result<(), Error> {
    let two_hours_in_the_future = now
        .checked_add_signed(Duration::hours(2))
        .ok_or("overflow when calculating 2 hours in the future")?;

    if block_header_time <= two_hours_in_the_future {
        Ok(())
    } else {
        Err("block header time is more than 2 hours in the future".into())
    }
}

/*
/// [3.10]: https://zips.z.cash/protocol/protocol.pdf#coinbasetransactions
pub(crate) fn coinbase_check(block: &Block) -> Result<(), Error> {
    if block.coinbase_height().is_some() {
        // No coinbase inputs in additional transactions allowed
        if block
            .transactions
            .iter()
            .skip(1)
            .any(|tx| tx.contains_coinbase_input())
        {
            Err("coinbase input found in additional transaction")?
        }
        Ok(())
    } else {
        Err("no coinbase transaction in block")?
    }
}
*/

struct BlockHeaderVerifier<S> {
    /// The underlying `ZebraState`, possibly wrapped in other services.
    state_service: S,
}

/// The error type for the BlockVerifier Service.
// TODO(jlusby): Error = Report ?
type Error = Box<dyn error::Error + Send + Sync + 'static>;

/// The BlockHeaderVerifier service implementation.
///
/// After verification, blocks are added to the underlying state service.
impl<S> Service<(Arc<BlockHeader>, BlockHeight)> for BlockHeaderVerifier<S>
where
    S: Service<zebra_state::RequestBlockHeader, Response = zebra_state::Response, Error = Error>
        + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
{
    type Response = (BlockHeaderHash, BlockHeight);
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        // We don't expect the state to exert backpressure on verifier users,
        // so we don't need to call `state_service.poll_ready()` here.
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, header: (Arc<BlockHeader>, BlockHeight)) -> Self::Future {
        // TODO(jlusby): Error = Report, handle errors from state_service.
        // TODO(teor):
        //   - handle chain reorgs
        //   - adjust state_service "unique block height" conditions
        let mut state_service = self.state_service.clone();

        let hash: BlockHeaderHash = header.0.as_ref().into();
        let hash_str = hex::encode(&hash.0);
        // let height = block.coinbase_height().unwrap();
        let block_header = header.0;
        let block_height = header.1;

        async move {
            // Since errors cause an early exit, try to do the
            // quick checks first.

            let now = Utc::now();
            node_time_check(block_header.time, now)?;
            block_header.is_equihash_solution_valid()?;
            // coinbase_check(block.as_ref())?; // didn't applicable for block headers handling

            // `Tower::Buffer` requires a 1:1 relationship between `poll()`s
            // and `call()`s, because it reserves a buffer slot in each
            // `call()`.
            let add_block_header = state_service
                .ready_and()
                .await?
                .call(zebra_state::RequestBlockHeader::AddBlockHeader { block_header, block_height });

            tracing::info!("Block header with height {:?} and hash {:?} stored!", block_height, hash_str);

            match add_block_header.await? {
                zebra_state::Response::Added { hash, height } => Ok((hash, height)),
                _ => Err("adding block header to zebra-state failed".into()),
            }
        }
        .boxed()
    }
}

/// Return a block header verification service, using the provided state service.
///
/// The block header verifier holds a state service of type `S`, used as context for
/// block header validation and to which newly verified block headers will be committed. This
/// state is pluggable to allow for testing or instrumentation.
///
/// The returned type is opaque to allow instrumentation or other wrappers, but
/// can be boxed for storage. It is also `Clone` to allow sharing of a
/// verification service.
///
/// This function should be called only once for a particular state service (and
/// the result be shared) rather than constructing multiple verification services
/// backed by the same state layer.
pub fn init<S>(
    state_service: S,
) -> impl Service<
    (Arc<BlockHeader>, BlockHeight),
    Response = (BlockHeaderHash, BlockHeight),
    Error = Error,
    Future = impl Future<Output = Result<(BlockHeaderHash, BlockHeight), Error>>,
> + Send
  + Clone
  + 'static
where
    S: Service<zebra_state::RequestBlockHeader, Response = zebra_state::Response, Error = Error>
        + Send
        + Clone
        + 'static,
    S::Future: Send + 'static,
{
    Buffer::new(BlockHeaderVerifier { state_service }, 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    use chrono::offset::{LocalResult, TimeZone};
    use chrono::{Duration, Utc};
    use color_eyre::eyre::Report;
    use color_eyre::eyre::{bail, eyre};
    use std::sync::Arc;
    use tower::{util::ServiceExt, Service};

    use zebra_chain::block::Block;
    use zebra_chain::block::BlockHeader;
    use zebra_chain::serialization::ZcashDeserialize;
    use zebra_chain::transaction::Transaction;

    #[test]
    fn time_check_past_block() {
        // This block is also verified as part of the BlockVerifier service
        // tests.
        let block =
            Arc::<Block>::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])
                .expect("block should deserialize");
        let now = Utc::now();

        // This check is non-deterministic, but BLOCK_MAINNET_415000 is
        // a long time in the past. So it's unlikely that the test machine
        // will have a clock that's far enough in the past for the test to
        // fail.
        node_time_check(block.header.time, now)
            .expect("the header time from a mainnet block should be valid");
    }

    #[test]
    fn time_check_now() {
        // These checks are deteministic, because all the times are offset
        // from the current time.
        let now = Utc::now();
        let three_hours_in_the_past = now - Duration::hours(3);
        let two_hours_in_the_future = now + Duration::hours(2);
        let two_hours_and_one_second_in_the_future =
            now + Duration::hours(2) + Duration::seconds(1);

        node_time_check(now, now).expect("the current time should be valid as a block header time");
        node_time_check(three_hours_in_the_past, now)
            .expect("a past time should be valid as a block header time");
        node_time_check(two_hours_in_the_future, now)
            .expect("2 hours in the future should be valid as a block header time");
        node_time_check(two_hours_and_one_second_in_the_future, now).expect_err(
            "2 hours and 1 second in the future should be invalid as a block header time",
        );

        // Now invert the tests
        // 3 hours in the future should fail
        node_time_check(now, three_hours_in_the_past)
            .expect_err("3 hours in the future should be invalid as a block header time");
        // The past should succeed
        node_time_check(now, two_hours_in_the_future)
            .expect("2 hours in the past should be valid as a block header time");
        node_time_check(now, two_hours_and_one_second_in_the_future)
            .expect("2 hours and 1 second in the past should be valid as a block header time");
    }

    /// Valid unix epoch timestamps for blocks, in seconds
    static BLOCK_HEADER_VALID_TIMESTAMPS: &[i64] = &[
        // These times are currently invalid DateTimes, but they could
        // become valid in future chrono versions
        i64::MIN,
        i64::MIN + 1,
        // These times are valid DateTimes
        (u32::MIN as i64) - 1,
        (u32::MIN as i64),
        (u32::MIN as i64) + 1,
        (i32::MIN as i64) - 1,
        (i32::MIN as i64),
        (i32::MIN as i64) + 1,
        -1,
        0,
        1,
        // maximum nExpiryHeight or lock_time, in blocks
        499_999_999,
        // minimum lock_time, in seconds
        500_000_000,
        500_000_001,
    ];

    /// Invalid unix epoch timestamps for blocks, in seconds
    static BLOCK_HEADER_INVALID_TIMESTAMPS: &[i64] = &[
        (i32::MAX as i64) - 1,
        (i32::MAX as i64),
        (i32::MAX as i64) + 1,
        (u32::MAX as i64) - 1,
        (u32::MAX as i64),
        (u32::MAX as i64) + 1,
        // These times are currently invalid DateTimes, but they could
        // become valid in future chrono versions
        i64::MAX - 1,
        i64::MAX,
    ];

    #[test]
    fn time_check_fixed() {
        // These checks are non-deterministic, but the times are all in the
        // distant past or far future. So it's unlikely that the test
        // machine will have a clock that makes these tests fail.
        let now = Utc::now();

        for valid_timestamp in BLOCK_HEADER_VALID_TIMESTAMPS {
            let block_header_time = match Utc.timestamp_opt(*valid_timestamp, 0) {
                LocalResult::Single(time) => time,
                LocalResult::None => {
                    // Skip the test if the timestamp is invalid
                    continue;
                }
                LocalResult::Ambiguous(_, _) => {
                    // Utc doesn't have ambiguous times
                    unreachable!();
                }
            };
            node_time_check(block_header_time, now)
                .expect("the time should be valid as a block header time");
            // Invert the check, leading to an invalid time
            node_time_check(now, block_header_time)
                .expect_err("the inverse comparison should be invalid");
        }

        for invalid_timestamp in BLOCK_HEADER_INVALID_TIMESTAMPS {
            let block_header_time = match Utc.timestamp_opt(*invalid_timestamp, 0) {
                LocalResult::Single(time) => time,
                LocalResult::None => {
                    // Skip the test if the timestamp is invalid
                    continue;
                }
                LocalResult::Ambiguous(_, _) => {
                    // Utc doesn't have ambiguous times
                    unreachable!();
                }
            };
            node_time_check(block_header_time, now)
                .expect_err("the time should be invalid as a block header time");
            // Invert the check, leading to a valid time
            node_time_check(now, block_header_time)
                .expect("the inverse comparison should be valid");
        }
    }

    #[tokio::test]
    async fn verify_test() -> Result<(), Report> {
        verify().await
    }

    #[spandoc::spandoc]
    async fn verify() -> Result<(), Report> {
        zebra_test::init();

        let block =
            Arc::<Block>::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])?;
        let block_header = block.header;
        let block_height = block.coinbase_height().unwrap();
        let hash: BlockHeaderHash = (&block_header).into();

        let state_service = zebra_state::in_memory_headersonly::init();
        let mut block_header_verifier = super::init(state_service.clone());

        /// SPANDOC: Make sure the verifier service is ready
        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Verify the block header
        let verify_response = ready_verifier_service
            .call((Arc::new(block_header.clone()), block_height))
            .await
            .map_err(|e| eyre!(e))?;

        assert_eq!(verify_response, (hash, block_height));

        Ok(())
    }

    #[tokio::test]
    async fn round_trip_test() -> Result<(), Report> {
        round_trip().await
    }

    #[spandoc::spandoc]
    async fn round_trip() -> Result<(), Report> {
        zebra_test::init();

        let block =
            Arc::<Block>::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])?;
        let block_header = block.header;
        let block_height = block.coinbase_height().unwrap();
        let hash: BlockHeaderHash = (&block_header).into();

        let mut state_service = zebra_state::in_memory_headersonly::init();
        let mut block_header_verifier = super::init(state_service.clone());

        /// SPANDOC: Make sure the verifier service is ready
        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Verify the block header
        let verify_response = ready_verifier_service
            .call((Arc::new(block_header.clone()), block_height))
            .await
            .map_err(|e| eyre!(e))?;

        assert_eq!(verify_response, (hash, block_height));

        /// SPANDOC: Make sure the state service is ready
        let ready_state_service = state_service.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Make sure the block header was added to the state
        let state_response = ready_state_service
            .call(zebra_state::RequestBlockHeader::GetBlockHeader { query: zebra_state::QueryType::ByHash(hash) })
            .await
            .map_err(|e| eyre!(e))?;

        if let zebra_state::Response::BlockHeader {
            block_header: returned_block_header,
            block_height: returned_block_height,
        } = state_response
        {
            assert_eq!(Arc::new(block_header.clone()), returned_block_header);
            assert_eq!(block_height, returned_block_height);
        } else {
            bail!("unexpected response kind: {:?}", state_response);
        }

        Ok(())
    }

    #[tokio::test]
    async fn verify_fail_add_block_test() -> Result<(), Report> {
        verify_fail_add_block().await
    }

    #[spandoc::spandoc]
    async fn verify_fail_add_block() -> Result<(), Report> {
        zebra_test::init();

        let block =
            Arc::<Block>::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])?;
        let block_header = block.header;
        let block_height = block.coinbase_height().unwrap();
        let hash: BlockHeaderHash = (&block_header).into();

        let mut state_service = zebra_state::in_memory_headersonly::init();
        let mut block_header_verifier = super::init(state_service.clone());

        /// SPANDOC: Make sure the verifier service is ready (1/2)
        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Verify the block for the first time
        let verify_response = ready_verifier_service
            .call((Arc::new(block_header.clone()), block_height))
            .await
            .map_err(|e| eyre!(e))?;

        assert_eq!(verify_response, (hash, block_height));

        /// SPANDOC: Make sure the state service is ready (1/2)
        let ready_state_service = state_service.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Make sure the block was added to the state
        let state_response = ready_state_service
            .call(zebra_state::RequestBlockHeader::GetBlockHeader { query: zebra_state::QueryType::ByHash(hash) })
            .await
            .map_err(|e| eyre!(e))?;

        if let zebra_state::Response::BlockHeader {
            block_header: returned_block_header,
            block_height: returned_block_height,
        } = state_response
        {
            assert_eq!(Arc::new(block_header.clone()), returned_block_header);
            assert_eq!(block_height, returned_block_height);
        } else {
            bail!("unexpected response kind: {:?}", state_response);
        }

        /// SPANDOC: Make sure the verifier service is ready (2/2)
        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Now try to add the block again, verify should fail
        // TODO(teor): ignore duplicate block verifies?
        // TODO(teor || jlusby): check error kind
        ready_verifier_service
            .call((Arc::new(block_header.clone()), block_height))
            .await
            .unwrap_err();

        /// SPANDOC: Make sure the state service is ready (2/2)
        let ready_state_service = state_service.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: But the state should still return the original block we added
        let state_response = ready_state_service
            .call(zebra_state::RequestBlockHeader::GetBlockHeader { query: zebra_state::QueryType::ByHash(hash) })
            .await
            .map_err(|e| eyre!(e))?;

        if let zebra_state::Response::BlockHeader {
            block_header: returned_block_header,
            block_height: returned_block_height,
        } = state_response
        {
            assert_eq!(Arc::new(block_header.clone()), returned_block_header);
            assert_eq!(block_height, returned_block_height);
        } else {
            bail!("unexpected response kind: {:?}", state_response);
        }

        Ok(())
    }

    #[tokio::test]
    async fn verify_fail_future_time_test() -> Result<(), Report> {
        verify_fail_future_time().await
    }

    #[spandoc::spandoc]
    async fn verify_fail_future_time() -> Result<(), Report> {
        zebra_test::init();

        let block =
            <Block>::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])?;
        let mut block_header = block.header;
        let block_height = block.coinbase_height().unwrap();

        let mut state_service = zebra_state::in_memory_headersonly::init();
        let mut block_header_verifier = super::init(state_service.clone());

        // Modify the block's time in block header
        // Changing the block header also invalidates the header hashes, but
        // those checks should be performed later in validation, because they
        // are more expensive.
        let three_hours_in_the_future = Utc::now()
            .checked_add_signed(Duration::hours(3))
            .ok_or("overflow when calculating 3 hours in the future")
            .map_err(|e| eyre!(e))?;
        block_header.time = three_hours_in_the_future;

        let arc_block_header: Arc<BlockHeader> = block_header.into();
        let  hash: BlockHeaderHash = arc_block_header.as_ref().into();

        /// SPANDOC: Make sure the verifier service is ready
        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Try to add the block, and expect failure
        // TODO(teor || jlusby): check error kind
        ready_verifier_service
            .call((arc_block_header.clone(), block_height))
            .await
            .unwrap_err();

        /// SPANDOC: Make sure the state service is ready (2/2)
        let ready_state_service = state_service.ready_and().await.map_err(|e| eyre!(e))?;
        /// SPANDOC: Now make sure the block isn't in the state
        // TODO(teor || jlusby): check error kind
        ready_state_service
            .call(zebra_state::RequestBlockHeader::GetBlockHeader {
                query: zebra_state::QueryType::ByHash(hash)
            })
            .await
            .unwrap_err();

        Ok(())
    }

    #[tokio::test]
    async fn header_solution_test() -> Result<(), Report> {
        header_solution().await
    }

    #[spandoc::spandoc]
    async fn header_solution() -> Result<(), Report> {
        zebra_test::init();

        // Get a valid block
        let block =
            Block::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])
                .expect("block test vector should deserialize");
        let mut block_header = block.header;
        let block_height = block.coinbase_height().unwrap();

        // Service variables
        let state_service = zebra_state::in_memory_headersonly::init();
        let mut block_header_verifier = super::init(state_service.clone());

        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;

        // This should be ok
        ready_verifier_service
            .call((Arc::new(block_header.clone()), block_height))
            .await
            .map_err(|e| eyre!(e))?;

        // Change nonce to something invalid
        block_header.nonce = [0; 32];

        let ready_verifier_service = block_header_verifier.ready_and().await.map_err(|e| eyre!(e))?;

        // Error: invalid equihash solution for BlockHeader
        ready_verifier_service
            .call((Arc::new(block_header.clone()), block_height))
            .await
            .expect_err("expected the equihash solution to be invalid");

        Ok(())
    }

    #[tokio::test]
    #[spandoc::spandoc]
    async fn coinbase() -> Result<(), Report> {
        zebra_test::init();

        // Service variables
        let state_service = zebra_state::in_memory::init();
//      let mut block_verifier = super::init(state_service.clone());
        let mut block_verifier = crate::verify::block::init(state_service.clone());

        // Get a header of a block
        let header =
            BlockHeader::zcash_deserialize(&zebra_test::vectors::DUMMY_HEADER[..]).unwrap();

        let ready_verifier_service = block_verifier.ready_and().await.map_err(|e| eyre!(e))?;

        // Test 1: Empty transaction
        let block = Block {
            header,
            transactions: Vec::new(),
        };

        // Error: no coinbase transaction in block
        ready_verifier_service
            .call(Arc::new(block.clone()))
            .await
            .expect_err("fail with no coinbase transaction in block");

        let ready_verifier_service = block_verifier.ready_and().await.map_err(|e| eyre!(e))?;

        // Test 2: Transaction at first position is not coinbase
        let mut transactions = Vec::new();
        let tx = Transaction::zcash_deserialize(&zebra_test::vectors::DUMMY_TX1[..]).unwrap();
        transactions.push(Arc::new(tx));
        let block = Block {
            header,
            transactions,
        };

        // Error: no coinbase transaction in block
        ready_verifier_service
            .call(Arc::new(block.clone()))
            .await
            .expect_err("fail with no coinbase transaction in block");

        let ready_verifier_service = block_verifier.ready_and().await.map_err(|e| eyre!(e))?;

        // Test 3: Invalid coinbase position
        let mut block =
            Block::zcash_deserialize(&zebra_test::vectors::BLOCK_MAINNET_415000_BYTES[..])?;
        assert_eq!(block.transactions.len(), 1);

        // Extract the coinbase transaction from the block
        let coinbase_transaction = block.transactions.get(0).unwrap().clone();

        // Add another coinbase transaction to block
        block.transactions.push(coinbase_transaction);
        assert_eq!(block.transactions.len(), 2);

        // Error: coinbase input found in additional transaction
        ready_verifier_service
            .call(Arc::new(block.clone()))
            .await
            .expect_err("fail with coinbase input found in additional transaction");

        Ok(())
    }
}

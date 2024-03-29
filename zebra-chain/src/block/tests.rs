use super::*;
use crate::equihash_solution::EquihashSolution;
use crate::merkle_tree::MerkleTreeRootHash;
use crate::note_commitment_tree::SaplingNoteTreeRootHash;
use crate::serialization::{
    SerializationError, ZcashDeserialize, ZcashDeserializeInto, ZcashSerialize,
};
use crate::{sha256d_writer::Sha256dWriter, test::generate};
use chrono::{TimeZone, Utc};
use proptest::{
    arbitrary::{any, Arbitrary},
    prelude::*,
};
use std::io::{Cursor, ErrorKind, Write};

impl Arbitrary for BlockHeader {
    type Parameters = ();

    fn arbitrary_with(_args: ()) -> Self::Strategy {
        (
            // version is interpreted as i32 in the spec, so we are limited to i32::MAX here
            (4u32..(i32::MAX as u32)),
            any::<BlockHeaderHash>(),
            any::<MerkleTreeRootHash>(),
            any::<SaplingNoteTreeRootHash>(),
            // time is interpreted as u32 in the spec, but rust timestamps are i64
            (0i64..(u32::MAX as i64)),
            any::<u32>(),
            any::<[u8; 32]>(),
            any::<EquihashSolution>(),
        )
            .prop_map(
                |(
                    version,
                    previous_block_hash,
                    merkle_root_hash,
                    final_sapling_root_hash,
                    timestamp,
                    bits,
                    nonce,
                    solution,
                )| BlockHeader {
                    version,
                    previous_block_hash,
                    merkle_root_hash,
                    final_sapling_root_hash,
                    time: Utc.timestamp(timestamp, 0),
                    bits,
                    nonce,
                    solution,
                },
            )
            .boxed()
    }

    type Strategy = BoxedStrategy<Self>;
}

#[test]
fn blockheaderhash_debug() {
    let preimage = b"foo bar baz";
    let mut sha_writer = Sha256dWriter::default();
    let _ = sha_writer.write_all(preimage);

    let hash = BlockHeaderHash(sha_writer.finish());

    assert_eq!(
        format!("{:?}", hash),
        "BlockHeaderHash(\"bf46b4b5030752fedac6f884976162bbfb29a9398f104a280b3e34d51b416631\")"
    );
}

#[test]
fn blockheaderhash_from_blockheader() {
    let blockheader = generate::block_header();

    let hash = BlockHeaderHash::from(&blockheader);

    assert_eq!(
        format!("{:?}", hash),
        "BlockHeaderHash(\"39c92b8c6b582797830827c78d58674c7205fcb21991887c124d1dbe4b97d6d1\")"
    );

    let mut bytes = Cursor::new(Vec::new());

    blockheader
        .zcash_serialize(&mut bytes)
        .expect("these bytes to serialize from a blockheader without issue");

    bytes.set_position(0);
    let other_header = bytes
        .zcash_deserialize_into()
        .expect("these bytes to deserialize into a blockheader without issue");

    assert_eq!(blockheader, other_header);
}

#[test]
fn deserialize_blockheader() {
    // https://explorer.zcha.in/blocks/415000
    let _header = zebra_test::vectors::HEADER_MAINNET_415000_BYTES
        .zcash_deserialize_into::<BlockHeader>()
        .expect("blockheader test vector should deserialize");
}

#[test]
fn deserialize_block() {
    zebra_test::vectors::BLOCK_MAINNET_GENESIS_BYTES
        .zcash_deserialize_into::<Block>()
        .expect("block test vector should deserialize");
    zebra_test::vectors::BLOCK_MAINNET_1_BYTES
        .zcash_deserialize_into::<Block>()
        .expect("block test vector should deserialize");
    // https://explorer.zcha.in/blocks/415000
    zebra_test::vectors::BLOCK_MAINNET_415000_BYTES
        .zcash_deserialize_into::<Block>()
        .expect("block test vector should deserialize");
    // https://explorer.zcha.in/blocks/434873
    // this one has a bad version field
    zebra_test::vectors::BLOCK_MAINNET_434873_BYTES
        .zcash_deserialize_into::<Block>()
        .expect("block test vector should deserialize");
}

#[test]
fn block_limits_multi_tx() {
    // Test multiple small transactions to fill a block max size

    // Create a block just below the limit
    let mut block = generate::large_multi_transaction_block();

    // Serialize the block
    let mut data = Vec::new();
    block
        .zcash_serialize(&mut data)
        .expect("block should serialize as we are not limiting generation yet");

    assert!(data.len() <= MAX_BLOCK_BYTES as usize);

    // Deserialize by now is ok as we are lower than the limit
    let block2 = Block::zcash_deserialize(&data[..])
        .expect("block should deserialize as we are just below limit");
    assert_eq!(block, block2);

    // Add 1 more transaction to the block, limit will be reached
    block = generate::oversized_multi_transaction_block();

    // Serialize will still be fine
    let mut data = Vec::new();
    block
        .zcash_serialize(&mut data)
        .expect("block should serialize as we are not limiting generation yet");

    assert!(data.len() > MAX_BLOCK_BYTES as usize);

    // Deserialize will now fail
    Block::zcash_deserialize(&data[..]).expect_err("block should not deserialize");
}

#[test]
fn block_limits_single_tx() {
    // Test block limit with a big single transaction

    // Create a block just below the limit
    let mut block = generate::large_single_transaction_block();

    // Serialize the block
    let mut data = Vec::new();
    block
        .zcash_serialize(&mut data)
        .expect("block should serialize as we are not limiting generation yet");

    assert!(data.len() <= MAX_BLOCK_BYTES as usize);

    // Deserialize by now is ok as we are lower than the limit
    Block::zcash_deserialize(&data[..])
        .expect("block should deserialize as we are just below limit");

    // Add 1 more input to the transaction, limit will be reached
    block = generate::oversized_single_transaction_block();

    let mut data = Vec::new();
    block
        .zcash_serialize(&mut data)
        .expect("block should serialize as we are not limiting generation yet");

    assert!(data.len() > MAX_BLOCK_BYTES as usize);

    // Will fail as block overall size is above limit
    Block::zcash_deserialize(&data[..]).expect_err("block should not deserialize");
}

proptest! {

    #[test]
    fn blockheaderhash_roundtrip(hash in any::<BlockHeaderHash>()) {
        let bytes = hash.zcash_serialize_to_vec()?;
        let other_hash = bytes.zcash_deserialize_into()?;

        prop_assert_eq![hash, other_hash];
    }

    #[test]
    fn blockheader_roundtrip(header in any::<BlockHeader>()) {
        let bytes = header.zcash_serialize_to_vec()?;
        let other_header = bytes.zcash_deserialize_into()?;

        prop_assert_eq![header, other_header];
    }

    #[test]
    fn block_roundtrip(block in any::<Block>()) {
        let bytes = block.zcash_serialize_to_vec()?;
        let bytes = &mut bytes.as_slice();

        // Check the block size limit
        if bytes.len() <= MAX_BLOCK_BYTES as _ {
            let other_block = bytes.zcash_deserialize_into()?;

            prop_assert_eq![block, other_block];
        } else {
            let serialization_err = bytes.zcash_deserialize_into::<Block>()
                .expect_err("blocks larger than the maximum size should fail");
            match serialization_err {
                SerializationError::Io(io_err) => {
                    prop_assert_eq![io_err.kind(), ErrorKind::UnexpectedEof];
                }
                _ => {
                    prop_assert!(false,
                                 "blocks larger than the maximum size should fail with an io::Error");
                }
            }
        }
    }

}

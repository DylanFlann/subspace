use crate::{extract_header_digests, verify_header_digest_with_parent, HeaderExt, ImportError};
use frame_support::{assert_err, assert_ok};
use sp_consensus_subspace::digests::{
    CompatibleDigestItem, GlobalRandomnessDescriptor, SaltDescriptor, SolutionRangeDescriptor,
};
use sp_runtime::traits::BlakeTwo256;
use sp_runtime::DigestItem;

type Header = sp_runtime::generic::Header<u32, BlakeTwo256>;

#[test]
fn test_header_digest_extraction() {
    let mut header = Header {
        parent_hash: [0u8; 32].into(),
        number: 1,
        state_root: Default::default(),
        extrinsics_root: Default::default(),
        digest: Default::default(),
    };

    let res = extract_header_digests(&header);
    assert_err!(res, ImportError::InvalidGlobalRandomnessDigest);

    let randomness = GlobalRandomnessDescriptor {
        global_randomness: Default::default(),
    };
    header
        .digest
        .logs
        .push(DigestItem::global_randomness_descriptor(randomness));
    let res = extract_header_digests(&header);
    assert_err!(res, ImportError::InvalidSolutionRangeDigest);

    let solution_range = SolutionRangeDescriptor { solution_range: 0 };
    header
        .digest
        .logs
        .push(DigestItem::solution_range_descriptor(solution_range));
    let res = extract_header_digests(&header);
    assert_err!(res, ImportError::InvalidSaltDigest);

    let salt = SaltDescriptor {
        salt: Default::default(),
    };
    header.digest.logs.push(DigestItem::salt_descriptor(salt));
    let res = extract_header_digests(&header);
    assert_ok!(res);
}

#[test]
fn verify_header_digests() {
    let expected_randomness = [1u8; 32];
    let expected_solution_range = 0;
    let expected_salt = [2u8; 8];

    let parent_header_ext = HeaderExt {
        header: Header {
            parent_hash: Default::default(),
            number: 0,
            state_root: Default::default(),
            extrinsics_root: Default::default(),
            digest: Default::default(),
        },
        derived_global_randomness: expected_randomness,
        derived_solution_range: expected_solution_range,
        derived_salt: expected_salt,
        total_weight: 0,
    };

    let mut header = Header {
        parent_hash: parent_header_ext.header.parent_hash,
        number: 1,
        state_root: Default::default(),
        extrinsics_root: Default::default(),
        digest: Default::default(),
    };
    let randomness = GlobalRandomnessDescriptor {
        global_randomness: expected_randomness,
    };
    header
        .digest
        .logs
        .push(DigestItem::global_randomness_descriptor(randomness));
    let solution_range = SolutionRangeDescriptor {
        solution_range: expected_solution_range,
    };
    header
        .digest
        .logs
        .push(DigestItem::solution_range_descriptor(solution_range));
    let salt = SaltDescriptor {
        salt: expected_salt,
    };
    header.digest.logs.push(DigestItem::salt_descriptor(salt));

    let res = verify_header_digest_with_parent(&parent_header_ext, &header);
    assert_ok!(res);
}

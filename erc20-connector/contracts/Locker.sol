pragma solidity ^0.8;

import "rainbow-bridge-sol/nearprover/contracts/INearProver.sol";
import "rainbow-bridge-sol/nearprover/contracts/ProofDecoder.sol";
import "rainbow-bridge-sol/nearbridge/contracts/Borsh.sol";

contract Locker {
    using Borsh for Borsh.Data;
    using ProofDecoder for Borsh.Data;

    INearProver public prover_;
    bytes public nearTokenFactory_;

    /// Proofs from blocks that are below the acceptance height will be rejected.
    // If `minBlockAcceptanceHeight_` value is zero - proofs from block with any height are accepted.
    uint64 public minBlockAcceptanceHeight_;

    // OutcomeReciptId -> Used
    mapping(bytes32 => bool) public usedProofs_;

    constructor(bytes memory nearTokenFactory, INearProver prover, uint64 minBlockAcceptanceHeight) public {
        require(nearTokenFactory.length > 0, "Invalid Near Token Factory address");
        require(address(prover) != address(0), "Invalid Near prover address");

        nearTokenFactory_ = nearTokenFactory;
        prover_ = prover;
        minBlockAcceptanceHeight_ = minBlockAcceptanceHeight;
    }

    /// Parses the provided proof and consumes it if it's not already used.
    /// The consumed event cannot be reused for future calls.
    function _parseAndConsumeProof(bytes memory proofData, uint64 proofBlockHeight)
        internal
        returns (ProofDecoder.ExecutionStatus memory result)
    {
        require(proofBlockHeight >= minBlockAcceptanceHeight_, "Proof is from the ancient block");
        require(prover_.proveOutcome(proofData, proofBlockHeight), "Proof should be valid");

        // Unpack the proof and extract the execution outcome.
        Borsh.Data memory borshData = Borsh.from(proofData);
        ProofDecoder.FullOutcomeProof memory fullOutcomeProof = borshData.decodeFullOutcomeProof();
        borshData.done();

        bytes32 receiptId = fullOutcomeProof.outcome_proof.outcome_with_id.outcome.receipt_ids[0];
        require(!usedProofs_[receiptId], "The burn event proof cannot be reused");
        usedProofs_[receiptId] = true;

        require(keccak256(fullOutcomeProof.outcome_proof.outcome_with_id.outcome.executor_id)
                == keccak256(nearTokenFactory_),
                "Can only unlock tokens from the linked proof producer on Near blockchain");

        result = fullOutcomeProof.outcome_proof.outcome_with_id.outcome.status;
        require(!result.failed, "Cannot use failed execution outcome for unlocking the tokens");
        require(!result.unknown, "Cannot use unknown execution outcome for unlocking the tokens");
    }
}

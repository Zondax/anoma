//! Implementation of the [`PrepareProposal`] ABCI++ method for the Shell

#[cfg(not(feature = "ABCI"))]
mod prepare_block {

    use super::super::*;
    use crate::node::ledger::shims::abcipp_shim_types::shim::TxBytes;

    impl<D, H> Shell<D, H>
    where
        D: DB + for<'iter> DBIter<'iter> + Sync + 'static,
        H: StorageHasher + Sync + 'static,
    {
        /// Begin a new block.
        ///
        /// We include half of the new wrapper txs given to us from the mempool
        /// by tendermint. The rest of the block is filled with decryptions
        /// of the wrapper txs from the previously committed block.
        ///
        /// INVARIANT: Any changes applied in this method must be reverted if
        /// the proposal is rejected (unless we can simply overwrite
        /// them in the next block).
        pub fn prepare_proposal(
            &mut self,
            req: RequestPrepareProposal,
        ) -> response::PrepareProposal {
            // We can safely reset meter, because if the block is rejected,
            // we'll reset again on the next proposal, until the
            // proposal is accepted
            self.gas_meter.reset();
            let txs = if let ShellMode::Validator { .. } = self.mode {
                // TODO: This should not be hardcoded
                let privkey = <EllipticCurve as PairingEngine>::G2Affine::prime_subgroup_generator();

                // filter in half of the new txs from Tendermint, only keeping
                // wrappers
                let number_of_new_txs = 1 + req.block_data.len() / 2;
                let mut txs: Vec<TxBytes> = req
                    .block_data
                    .into_iter()
                    .take(number_of_new_txs)
                    .filter(|tx_bytes| {
                        if let Ok(tx) = Tx::try_from(tx_bytes.as_slice()) {
                            matches!(process_tx(tx), Ok(TxType::Wrapper(_)))
                        } else {
                            false
                        }
                    })
                    .collect();

                // decrypt the wrapper txs included in the previous block
                let mut decrypted_txs = self
                    .storage
                    .tx_queue
                    .iter()
                    .map(|tx| {
                        Tx::from(match tx.decrypt(privkey) {
                            Ok(tx) => DecryptedTx::Decrypted(tx),
                            _ => DecryptedTx::Undecryptable(tx.clone()),
                        })
                        .to_bytes()
                    })
                    .collect();

                txs.append(&mut decrypted_txs);
                txs
            } else {
                vec![]
            };
            response::PrepareProposal { block_data: txs }
        }
    }

    #[cfg(test)]
    mod test_prepare_proposal {
        use anoma::types::address::xan;
        use anoma::types::storage::Epoch;
        use anoma::types::transaction::Fee;

        use super::*;
        use crate::node::ledger::shell::test_utils::{gen_keypair, TestShell};

        /// Test that if a tx from the mempool is not a
        /// WrapperTx type, it is not included in the
        /// proposed block.
        #[test]
        fn test_prepare_proposal_rejects_non_wrapper_tx() {
            let (mut shell, _) = TestShell::new();
            let tx = Tx::new(
                "wasm_code".as_bytes().to_owned(),
                Some("transaction_data".as_bytes().to_owned()),
            );
            let req = RequestPrepareProposal {
                block_data: vec![tx.to_bytes()],
                block_data_size: 0,
            };
            assert_eq!(shell.prepare_proposal(req).block_data.len(), 0);
        }

        /// Test that if an error is encountered while
        /// trying to process a tx from the mempool,
        /// we simply exclude it from the proposal
        #[test]
        fn test_error_in_processing_tx() {
            let (mut shell, _) = TestShell::new();
            let keypair = gen_keypair();
            let tx = Tx::new(
                "wasm_code".as_bytes().to_owned(),
                Some("transaction_data".as_bytes().to_owned()),
            );
            // an unsigned wrapper will cause an error in processing
            let wrapper = Tx::new(
                "".as_bytes().to_owned(),
                Some(
                    WrapperTx::new(
                        Fee {
                            amount: 0.into(),
                            token: xan(),
                        },
                        &keypair,
                        Epoch(0),
                        0.into(),
                        tx,
                        Default::default(),
                    )
                    .try_to_vec()
                    .expect("Test failed"),
                ),
            )
            .to_bytes();
            let req = RequestPrepareProposal {
                block_data: vec![wrapper],
                block_data_size: 0,
            };
            assert_eq!(shell.prepare_proposal(req).block_data.len(), 0);
        }

        /// Test that the decrypted txs are included
        /// in the proposal in the same order as their
        /// corresponding wrappers
        #[test]
        fn test_decrypted_txs_in_correct_order() {
            let (mut shell, _) = TestShell::new();
            let keypair = gen_keypair();
            let mut expected_wrapper = vec![];
            let mut expected_decrypted = vec![];

            let mut req = RequestPrepareProposal {
                block_data: vec![],
                block_data_size: 0,
            };
            // create a request with two new wrappers from mempool and
            // two wrappers from the previous block to be decrypted
            for i in 0..2 {
                let tx = Tx::new(
                    "wasm_code".as_bytes().to_owned(),
                    Some(
                        format!("transaction data: {}", i)
                            .as_bytes()
                            .to_owned(),
                    ),
                );
                expected_decrypted
                    .push(Tx::from(DecryptedTx::Decrypted(tx.clone())));
                let wrapper_tx = WrapperTx::new(
                    Fee {
                        amount: 0.into(),
                        token: xan(),
                    },
                    &keypair,
                    Epoch(0),
                    0.into(),
                    tx,
                    Default::default(),
                );
                let wrapper = wrapper_tx.sign(&keypair).expect("Test failed");
                shell.enqueue_tx(wrapper_tx);
                expected_wrapper.push(wrapper.clone());
                req.block_data.push(wrapper.to_bytes());
            }
            // we extract the inner data from the txs for testing
            // equality since otherwise changes in timestamps would
            // fail the test
            expected_wrapper.append(&mut expected_decrypted);
            let expected_txs: Vec<Vec<u8>> = expected_wrapper
                .iter()
                .map(|tx| tx.data.clone().expect("Test failed"))
                .collect();

            let received: Vec<Vec<u8>> = shell
                .prepare_proposal(req)
                .block_data
                .iter()
                .map(|tx_bytes| {
                    Tx::try_from(tx_bytes.as_slice())
                        .expect("Test failed")
                        .data
                        .expect("Test failed")
                })
                .collect();
            // check that the order of the txs is correct
            assert_eq!(received, expected_txs);
        }
    }
}

#[cfg(not(feature = "ABCI"))]
pub use prepare_block::*;

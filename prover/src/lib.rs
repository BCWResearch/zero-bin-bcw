use std::future::Future;
use std::time::{Duration, Instant};

use alloy::primitives::U256;
use anyhow::Result;
use chrono::{DateTime, Utc};
use futures::{future::BoxFuture, stream::FuturesOrdered, FutureExt, TryFutureExt, TryStreamExt};
use num_traits::ToPrimitive as _;
use ops::TxProof;
use paladin::{
    directive::{Directive, IndexedStream},
    runtime::Runtime,
};
use proof_gen::proof_types::GeneratedBlockProof;
use serde::{Deserialize, Serialize};
use tokio::sync::oneshot;
use trace_decoder::{
    processed_block_trace::ProcessingMeta,
    trace_protocol::BlockTrace,
    types::{CodeHash, OtherBlockData},
};
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
pub struct BlockProverInput {
    pub block_trace: BlockTrace,
    pub other_data: OtherBlockData,
}
fn resolve_code_hash_fn(_: &CodeHash) -> Vec<u8> {
    todo!()
}
#[derive(Debug, Clone)]
pub struct BenchmarkedGeneratedBlockProof {
    pub proof: GeneratedBlockProof,
    pub prep_dur: Option<Duration>,
    pub proof_dur: Option<Duration>,
    pub agg_dur: Option<Duration>,
    pub total_dur: Option<Duration>,
    pub n_txs: u64,
    pub gas_used: u64,
    pub gas_used_per_tx: Vec<u64>,
    pub difficulty: u64,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
}

impl From<BenchmarkedGeneratedBlockProof> for GeneratedBlockProof {
    fn from(value: BenchmarkedGeneratedBlockProof) -> Self {
        value.proof
    }
}

impl BlockProverInput {
    pub fn get_block_number(&self) -> U256 {
        self.other_data.b_data.b_meta.block_number.into()
    }

    /// Evaluates a singular block
    #[cfg(not(feature = "test_only"))]
    pub async fn prove_and_benchmark(
        self,
        runtime: &Runtime,
        previous: Option<impl Future<Output = Result<BenchmarkedGeneratedBlockProof>>>,
        save_inputs_on_error: bool,
    ) -> Result<BenchmarkedGeneratedBlockProof> {
        // Start timing for preparation
        let prep_start = Instant::now();
        let start_time: DateTime<Utc> = Utc::now();

        // Basic preparation
        let block_number = self.get_block_number();
        let other_data = self.other_data;
        let txs = self.block_trace.into_txn_proof_gen_ir(
            &ProcessingMeta::new(resolve_code_hash_fn),
            other_data.clone(),
        )?;

        let n_txs = txs.len();
        let gas_used = u64::try_from(other_data.b_data.b_meta.block_gas_used).expect("Overflow");
        let gas_used_per_tx = txs
            .iter()
            .map(|tx| {
                u64::try_from(tx.gas_used_after - tx.gas_used_before).expect("Overflow of gas")
            })
            .collect();
        let difficulty = other_data.b_data.b_meta.block_difficulty;

        // Get time took to prepare
        let prep_dur = prep_start.elapsed();

        info!(
            "Completed pre-proof work for block {} in {} secs",
            block_number,
            prep_dur.as_secs_f64()
        );

        // Time the agg proof
        let proof_start = Instant::now();
        let agg_proof = IndexedStream::from(txs)
            .map(&TxProof {
                save_inputs_on_error,
            })
            .fold(&ops::AggProof {
                save_inputs_on_error,
            })
            .run(runtime)
            .await?;
        let proof_dur = proof_start.elapsed();

        info!(
            "Completed tx proofs for block {} in {} secs",
            block_number,
            proof_dur.as_secs_f64()
        );

        //
        if let proof_gen::proof_types::AggregatableProof::Agg(proof) = agg_proof {
            let agg_start = Instant::now();
            let prev = match previous {
                Some(it) => Some(it.await?),
                None => None,
            };

            let block_proof = paladin::directive::Literal(proof)
                .map(&ops::BlockProof {
                    prev: prev.map(|prev| prev.proof),
                    save_inputs_on_error,
                })
                .run(runtime)
                .await?;

            let agg_dur = agg_start.elapsed();

            info!(
                "Completed tx proof agg for block {} in {} secs",
                block_number,
                agg_dur.as_secs_f64()
            );

            let end_time: DateTime<Utc> = Utc::now();

            // Return the block proof
            Ok(BenchmarkedGeneratedBlockProof {
                proof: block_proof.0,
                total_dur: Some(prep_start.elapsed()),
                proof_dur: Some(proof_dur),
                prep_dur: Some(prep_dur),
                agg_dur: Some(agg_dur),
                n_txs: n_txs as u64,
                gas_used,
                gas_used_per_tx,
                difficulty: u64::try_from(difficulty).expect("Difficulty overflow"),
                start_time,
                end_time,
            })
        } else {
            anyhow::bail!("AggProof is is not GeneratedAggProof")
        }
    }

    /// Evaluates a singular block
    #[cfg(not(feature = "test_only"))]
    pub async fn prove(
        self,
        runtime: &Runtime,
        previous: Option<impl Future<Output = Result<GeneratedBlockProof>>>,
        save_inputs_on_error: bool,
    ) -> Result<GeneratedBlockProof> {
        use anyhow::Context as _;

        let block_number = self.get_block_number();
        let other_data = self.other_data;
        let txs = self.block_trace.into_txn_proof_gen_ir(
            &ProcessingMeta::new(resolve_code_hash_fn),
            other_data.clone(),
        )?;

        let agg_proof = IndexedStream::from(txs)
            .map(&TxProof {
                save_inputs_on_error,
            })
            .fold(&ops::AggProof {
                save_inputs_on_error,
            })
            .run(runtime)
            .await?;

        if let proof_gen::proof_types::AggregatableProof::Agg(proof) = agg_proof {
            let _block_number = block_number
                .to_u64()
                .context("block number overflows u64")?;
            let prev = match previous {
                Some(it) => Some(it.await?),
                None => None,
            };

            let block_proof = paladin::directive::Literal(proof)
                .map(&ops::BlockProof {
                    prev,
                    save_inputs_on_error,
                })
                .run(runtime)
                .await?;

            // Return the block proof
            Ok(block_proof.0)
        } else {
            anyhow::bail!("AggProof is is not GeneratedAggProof")
        }
    }

    #[cfg(feature = "test_only")]
    pub async fn prove(
        self,
        runtime: &Runtime,
        _previous: Option<impl Future<Output = Result<GeneratedBlockProof>>>,
        save_inputs_on_error: bool,
    ) -> Result<GeneratedBlockProof> {
        let block_number = self.get_block_number();
        info!("Testing witness generation for block {block_number}.");

        let other_data = self.other_data;
        let txs = self.block_trace.into_txn_proof_gen_ir(
            &ProcessingMeta::new(resolve_code_hash_fn),
            other_data.clone(),
        )?;

        IndexedStream::from(txs)
            .map(&TxProof {
                save_inputs_on_error,
            })
            .run(runtime)
            .await?
            .try_collect::<Vec<_>>()
            .await?;

        info!("Successfully generated witness for block {block_number}.");

        // Dummy proof to match expected output type.
        Ok(GeneratedBlockProof {
            b_height: block_number
                .to_u64()
                .expect("Block number should fit in a u64"),
            intern: proof_gen::proof_gen::dummy_proof()?,
        })
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ProverInput {
    pub blocks: Vec<BlockProverInput>,
}

impl ProverInput {
    pub async fn prove_and_benchmark(
        self,
        runtime: &Runtime,
        previous_proof: Option<BenchmarkedGeneratedBlockProof>,
        save_inputs_on_error: bool,
    ) -> Result<Vec<BenchmarkedGeneratedBlockProof>> {
        let mut prev: Option<BoxFuture<Result<BenchmarkedGeneratedBlockProof>>> =
            previous_proof.map(|proof| Box::pin(futures::future::ok(proof)) as BoxFuture<_>);

        let results: FuturesOrdered<_> = self
            .blocks
            .into_iter()
            .map(|block| {
                let block_number = block.get_block_number();
                info!("Proving block {block_number}");

                let (tx, rx) = oneshot::channel::<BenchmarkedGeneratedBlockProof>();

                let fut = block
                    .prove_and_benchmark(runtime, prev.take(), save_inputs_on_error)
                    .then(|proof| async {
                        let proof = proof?;

                        if tx.send(proof.clone()).is_err() {
                            anyhow::bail!("Failed to send proof");
                        }

                        Ok(proof)
                    })
                    .boxed();

                prev = Some(Box::pin(rx.map_err(anyhow::Error::new)));

                fut
            })
            .collect();

        results.try_collect().await
    }

    pub async fn prove(
        self,
        runtime: &Runtime,
        previous_proof: Option<GeneratedBlockProof>,
        save_inputs_on_error: bool,
    ) -> Result<Vec<GeneratedBlockProof>> {
        let mut prev: Option<BoxFuture<Result<GeneratedBlockProof>>> =
            previous_proof.map(|proof| Box::pin(futures::future::ok(proof)) as BoxFuture<_>);

        let results: FuturesOrdered<_> = self
            .blocks
            .into_iter()
            .map(|block| {
                let block_number = block.get_block_number();
                info!("Proving block {block_number}");

                let (tx, rx) = oneshot::channel::<GeneratedBlockProof>();

                // Prove the block
                let fut = block
                    .prove(runtime, prev.take(), save_inputs_on_error)
                    .then(|proof| async {
                        let proof = proof?;

                        if tx.send(proof.clone()).is_err() {
                            anyhow::bail!("Failed to send proof");
                        }

                        Ok(proof)
                    })
                    .boxed();

                prev = Some(Box::pin(rx.map_err(anyhow::Error::new)));

                fut
            })
            .collect();

        results.try_collect().await
    }
}

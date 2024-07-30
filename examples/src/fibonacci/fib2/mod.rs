// Copyright (c) Facebook, Inc. and its affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

use super::utils::compute_fib_term;
use crate::{Blake3_192, Blake3_256, Example, ExampleOptions, HashFunction, Sha3_256};
use core::marker::PhantomData;
use std::time::Instant;
use tracing::{field, info_span};
use winterfell::{
    crypto::{DefaultRandomCoin, ElementHasher},
    math::{fields::f128::BaseElement, FieldElement},
    ProofOptions, Prover, StarkProof, Trace, VerifierError,
};

mod air;
use air::FibAir;

mod prover;
use prover::FibProver;

#[cfg(test)]
mod tests;

// CONSTANTS
// ================================================================================================

const TRACE_WIDTH: usize = 2;

// FIBONACCI EXAMPLE
// ================================================================================================

pub fn get_example(
    options: &ExampleOptions,
    sequence_length: usize,
) -> Result<Box<dyn Example>, String> {
    let (options, hash_fn) = options.to_proof_options(28, 8);

    match hash_fn {
        HashFunction::Blake3_192 => {
            Ok(Box::new(FibExample::<Blake3_192>::new(sequence_length, options)))
        }
        HashFunction::Blake3_256 => {
            Ok(Box::new(FibExample::<Blake3_256>::new(sequence_length, options)))
        }
        HashFunction::Sha3_256 => {
            Ok(Box::new(FibExample::<Sha3_256>::new(sequence_length, options)))
        }
        _ => Err("The specified hash function cannot be used with this example.".to_string()),
    }
}

pub struct FibExample<H: ElementHasher> {
    options: ProofOptions,
    sequence_length: usize,
    result: BaseElement,
    _hasher: PhantomData<H>,
}

impl<H: ElementHasher> FibExample<H> {
    pub fn new(sequence_length: usize, options: ProofOptions) -> Self {
        assert!(sequence_length.is_power_of_two(), "sequence length must be a power of 2");

        // compute Fibonacci sequence
        let now = Instant::now();
        let result = compute_fib_term(sequence_length);
        println!(
            "Computed Fibonacci sequence up to {}th term in {} ms: {}",
            sequence_length,
            now.elapsed().as_millis(),
            result,
        );

        FibExample {
            options,
            sequence_length,
            result,
            _hasher: PhantomData,
        }
    }
}

// EXAMPLE IMPLEMENTATION
// ================================================================================================

impl<H: ElementHasher> Example for FibExample<H>
where
    H: ElementHasher<BaseField = BaseElement>,
{
    fn prove(&self) -> StarkProof {
        println!(
            "Generating proof for computing Fibonacci sequence (2 terms per step) up to {}th term",
            self.sequence_length
        );

        // create a prover
        let prover = FibProver::<H>::new(self.options.clone());

        // generate execution trace
        let trace =
            info_span!("generate_execution_trace", num_cols = TRACE_WIDTH, steps = field::Empty)
                .in_scope(|| {
                    let trace = prover.build_trace(self.sequence_length);
                    tracing::Span::current().record("steps", trace.length());
                    trace
                });

        // generate the proof
        prover.prove(trace).unwrap()
    }

    fn verify(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let acceptable_options =
            winterfell::AcceptableOptions::OptionSet(vec![proof.options().clone()]);

        winterfell::verify::<FibAir, H, DefaultRandomCoin<H>>(
            proof,
            self.result,
            &acceptable_options,
        )
    }

    fn verify_with_wrong_inputs(&self, proof: StarkProof) -> Result<(), VerifierError> {
        let acceptable_options =
            winterfell::AcceptableOptions::OptionSet(vec![proof.options().clone()]);
        winterfell::verify::<FibAir, H, DefaultRandomCoin<H>>(
            proof,
            self.result + BaseElement::ONE,
            &acceptable_options,
        )
    }
}

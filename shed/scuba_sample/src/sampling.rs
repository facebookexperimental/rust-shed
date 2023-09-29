/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is licensed under both the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree and the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree.
 */

use std::num::NonZeroU64;

use rand::Rng;

use crate::sample::ScubaSample;

/// Indicates the status of this particular sample with regard to sampling.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Sampling {
    /// This sample has not had any sampling applied to it.
    NoSampling,
    /// This sample has had sampling applied to it and should be logged. The value represents the
    /// inverse of the probability that it would be sampled in (i.e. how many hits this sample
    /// should account for).
    SampledIn(NonZeroU64),
    /// This sample has had sampling applied to it and should not be logged.
    SampledOut,
}

/// Indicates whether a sample should be logged to Scuba
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ShouldLog {
    /// The sample should be sent to Scuba due to its sampling result.
    Log,
    /// The sample should not be sent to Scuba due to its sampling result.
    DoNotLog,
}

impl Sampling {
    /// Apply a sampling decision to this Sampling instance, using the provided sample_rate. One in
    /// sample_rate samples will be sampled in.
    pub fn sample<R: Rng>(&self, rng: &mut R, sample_rate: NonZeroU64) -> Self {
        let val = rng.gen_range(0..sample_rate.get());

        if val == 0 {
            // Sample it in!
            return match self {
                Self::NoSampling => Self::SampledIn(sample_rate),
                Self::SampledIn(r) => {
                    let new_sample_rate = NonZeroU64::new(sample_rate.get() * r.get())
                        .expect("Product of NonZeroU64 should be non-zero");
                    Self::SampledIn(new_sample_rate)
                }
                Self::SampledOut => Self::SampledOut,
            };
        }

        // Otherwise, sample it out.
        Self::SampledOut
    }

    /// Indicate whether a given [ScubaSample] should be logged, and modifies the sample
    /// accordingly to report that it has been sampled.
    pub fn apply(&self, sample: &mut ScubaSample) -> ShouldLog {
        match &self {
            Self::NoSampling => ShouldLog::Log,
            Self::SampledIn(r) => {
                // Notify the backend that sampling has happened.
                sample.add("sample_rate", r.get());
                ShouldLog::Log
            }
            Self::SampledOut => ShouldLog::DoNotLog,
        }
    }

    /// Indicate whether this [Sampling] will require logging when applied.
    pub fn is_logged(&self) -> ShouldLog {
        match &self {
            Self::NoSampling => ShouldLog::Log,
            Self::SampledIn(..) => ShouldLog::Log,
            Self::SampledOut => ShouldLog::DoNotLog,
        }
    }
}

#[cfg(test)]
mod test {
    use nonzero_ext::nonzero;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng; // Used for deterministic rng.

    use super::*;
    use crate::value::ScubaValue;

    #[test]
    fn test_sampled_in() {
        let mut rng = ChaCha8Rng::seed_from_u64(1);

        let sampling = Sampling::NoSampling.sample(&mut rng, nonzero!(2u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(2u64)));

        let sampling = sampling.sample(&mut rng, nonzero!(3u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(6u64)));
    }

    #[test]
    fn test_sampled_out() {
        let mut rng = ChaCha8Rng::seed_from_u64(1);

        let sampling = Sampling::NoSampling.sample(&mut rng, nonzero!(1u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(1u64)));

        let sampling = sampling.sample(&mut rng, nonzero!(2u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(2u64)));

        let sampling = sampling.sample(&mut rng, nonzero!(10u64));
        assert_eq!(sampling, Sampling::SampledOut);
    }

    #[test]
    fn test_add_sample_rate() {
        let mut sample = ScubaSample::new();
        let sampling = Sampling::SampledIn(nonzero!(10u64));

        assert_eq!(sampling.apply(&mut sample), ShouldLog::Log);
        assert_eq!(sample.get("sample_rate"), Some(&ScubaValue::Int(10)));
    }

    #[test]
    fn test_is_logged() {
        assert_eq!(Sampling::NoSampling.is_logged(), ShouldLog::Log);
        assert_eq!(
            Sampling::SampledIn(nonzero!(1u64)).is_logged(),
            ShouldLog::Log
        );
        assert_eq!(Sampling::SampledOut.is_logged(), ShouldLog::DoNotLog);
    }
}

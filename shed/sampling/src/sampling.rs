/*
 * Copyright (c) Meta Platforms, Inc. and affiliates.
 *
 * This source code is dual-licensed under either the MIT license found in the
 * LICENSE-MIT file in the root directory of this source tree or the Apache
 * License, Version 2.0 found in the LICENSE-APACHE file in the root directory
 * of this source tree. You may select, at your option, one of the
 * above-listed licenses.
 */

use std::num::NonZeroU64;

use rand::Rng;

use crate::SampleResult;
use crate::Sampleable;

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
    SampledOut(NonZeroU64),
}

impl Sampling {
    /// Apply a sampling decision to this Sampling instance, using the provided sample_rate. One in
    /// sample_rate samples will be sampled in.
    #[must_use]
    pub fn subsampled<R: Rng>(&self, rng: &mut R, sample_rate: NonZeroU64) -> Self {
        let val = rng.gen_range(0..sample_rate.get());

        let previous_sample_rate = match self {
            Self::NoSampling => const { NonZeroU64::new(1).unwrap() },
            Self::SampledIn(r) | Self::SampledOut(r) => *r,
        };

        let new_sample_rate = NonZeroU64::new(previous_sample_rate.get() * sample_rate.get())
            .expect("Product of NonZeroU64 should be non-zero");

        if val == 0 {
            // Sample it in!
            return match self {
                Self::NoSampling => Self::SampledIn(new_sample_rate),
                Self::SampledIn(_) => Self::SampledIn(new_sample_rate),
                Self::SampledOut(_) => Self::SampledOut(new_sample_rate),
            };
        }

        // Otherwise, sample it out.
        Self::SampledOut(new_sample_rate)
    }

    /// Indicate whether a given sample should be logged, and modifies the sample
    /// accordingly to report that it has been sampled.
    pub fn apply(&self, sample: &mut impl Sampleable) -> SampleResult {
        match &self {
            Self::NoSampling => SampleResult::Include,
            Self::SampledIn(r) => {
                // Notify the backend that sampling has happened.
                sample.set_sample_rate(*r);
                SampleResult::Include
            }
            Self::SampledOut(..) => SampleResult::Exclude,
        }
    }

    /// Indicate whether this [Sampling] will require logging when applied.
    pub fn to_result(&self) -> SampleResult {
        match &self {
            Self::NoSampling => SampleResult::Include,
            Self::SampledIn(..) => SampleResult::Include,
            Self::SampledOut(..) => SampleResult::Exclude,
        }
    }
}

#[cfg(test)]
mod test {
    use nonzero_ext::nonzero;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng; // Used for deterministic rng.

    use super::*;

    #[derive(Debug, Default)]
    struct TestSample {
        pub sample_rate: Option<NonZeroU64>,
    }

    impl Sampleable for TestSample {
        fn set_sample_rate(&mut self, sample_rate: NonZeroU64) {
            self.sample_rate = Some(sample_rate);
        }
    }

    #[test]
    fn test_sampled_in() {
        let mut rng = ChaCha8Rng::seed_from_u64(1);

        let sampling = Sampling::NoSampling.subsampled(&mut rng, nonzero!(2u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(2u64)));

        let sampling = sampling.subsampled(&mut rng, nonzero!(3u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(6u64)));
    }

    #[test]
    fn test_sampled_out() {
        let mut rng = ChaCha8Rng::seed_from_u64(1);

        let sampling = Sampling::NoSampling.subsampled(&mut rng, nonzero!(1u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(1u64)));

        let sampling = sampling.subsampled(&mut rng, nonzero!(2u64));
        assert_eq!(sampling, Sampling::SampledIn(nonzero!(2u64)));

        let sampling = sampling.subsampled(&mut rng, nonzero!(10u64));
        assert_eq!(sampling, Sampling::SampledOut(nonzero!(20u64)));
    }

    #[test]
    fn test_add_sample_rate() {
        let mut sample = TestSample::default();
        let sampling = Sampling::SampledIn(nonzero!(10u64));

        assert_eq!(sampling.apply(&mut sample), SampleResult::Include);
        assert_eq!(sample.sample_rate, Some(nonzero!(10u64)));
    }

    #[test]
    fn test_is_logged() {
        assert_eq!(Sampling::NoSampling.to_result(), SampleResult::Include);
        assert_eq!(
            Sampling::SampledIn(nonzero!(1u64)).to_result(),
            SampleResult::Include
        );
        assert_eq!(
            Sampling::SampledOut(nonzero!(1u64)).to_result(),
            SampleResult::Exclude
        );
    }
}

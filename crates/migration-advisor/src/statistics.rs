//! Paired bootstrap of mean latency gain, with a declared circular block length for
//! serial dependence. Choosing that length still requires evidence outside this module.
use crate::advisor::Pair;
use serde::{Deserialize, Serialize};

const MIN_RESAMPLING_BLOCKS: usize = 30;
const MIN_RESAMPLES: usize = 2000;
const MAX_RESAMPLES: usize = 10000;
pub const MAX_PAIRS: usize = 4096;
const P95_TAIL_MINIMUM: usize = 100;
const P99_TAIL_MINIMUM: usize = 1000;
const MIN_COMPARISON_TAIL_RESAMPLES: usize = 20;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StatisticalPolicy {
    pub min_resampling_blocks: usize,
    /// Circular moving-block length over ordered whole-run pairs; 1 is the i.i.d. bootstrap.
    pub resampling_block_length: usize,
    pub bootstrap_resamples: usize,
    pub confidence_basis_points: u32,
    pub seed: u64,
    pub improvement_margin_ns: u64,
    pub p95_regression_budget_ns: u64,
    pub p99_regression_budget_ns: Option<u64>,
}
impl StatisticalPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_RESAMPLING_BLOCKS..=MAX_PAIRS).contains(&self.min_resampling_blocks)
            || self.resampling_block_length == 0
            || self.resampling_block_length > self.min_resampling_blocks / 2
            || !(MIN_RESAMPLES..=MAX_RESAMPLES).contains(&self.bootstrap_resamples)
            || !(9000..=9900).contains(&self.confidence_basis_points)
        {
            return Err(
                "invalid predeclared effective-sample/block/bootstrap/confidence policy".into(),
            );
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Distribution {
    pub median_ns: u64,
    pub p95_ns: Option<u64>,
    pub p99_ns: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Statistics {
    pub paired_runs: usize,
    pub resampling_block_length: usize,
    pub resampling_blocks: usize,
    pub comparison_family_size: usize,
    pub comparison_confidence_basis_points: f64,
    pub comparison_tail_resamples: usize,
    pub baseline: Distribution,
    pub candidate: Distribution,
    pub mean_gain_ns: f64,
    pub mean_gain_fraction: f64,
    pub gain_interval_ns: [f64; 2],
    pub confidence_basis_points: u32,
    pub bootstrap_resamples: usize,
    pub seed: u64,
    pub decision: String,
}

fn distribution(mut values: Vec<u64>, effective_units: usize) -> Distribution {
    values.sort_unstable();
    let rank = |percent: usize| values[(percent * values.len()).div_ceil(100) - 1];
    Distribution {
        median_ns: rank(50),
        p95_ns: (effective_units >= P95_TAIL_MINIMUM).then(|| rank(95)),
        p99_ns: (effective_units >= P99_TAIL_MINIMUM).then(|| rank(99)),
    }
}

// SplitMix64 provides reproducible resampling, not security entropy. Rejection avoids modulo bias.
fn index(state: &mut u64, length: usize) -> usize {
    loop {
        *state = state.wrapping_add(0x9e3779b97f4a7c15);
        let mut value = *state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
        value ^= value >> 31;
        let bound = length as u64;
        if value >= bound.wrapping_neg() % bound {
            return (value % bound) as usize;
        }
    }
}

pub fn analyze(pairs: &[Pair], policy: &StatisticalPolicy) -> Result<Statistics, String> {
    analyze_family(pairs, policy, 1)
}

/// Bonferroni-adjusted paired bootstrap interval for a predeclared family of candidate plans.
/// The family size is the number of matched benchmark hypotheses in the request.
pub fn analyze_family(
    pairs: &[Pair],
    policy: &StatisticalPolicy,
    family_size: usize,
) -> Result<Statistics, String> {
    policy.validate()?;
    let resampling_blocks = pairs.len() / policy.resampling_block_length;
    if family_size == 0
        || resampling_blocks < policy.min_resampling_blocks
        || pairs.len() > MAX_PAIRS
        || pairs
            .iter()
            .any(|p| p.baseline_ns == 0 || p.candidate_ns == 0)
    {
        return Err("insufficient resampling blocks or excessive/invalid paired runs".into());
    }
    let gains: Vec<_> = pairs
        .iter()
        .map(|p| (i128::from(p.baseline_ns) - i128::from(p.candidate_ns)) as f64)
        .collect();
    let mean = gains.iter().sum::<f64>() / pairs.len() as f64;
    let baseline_mean =
        pairs.iter().map(|p| p.baseline_ns as f64).sum::<f64>() / pairs.len() as f64;
    let mut state = policy.seed;
    let mut resamples = Vec::with_capacity(policy.bootstrap_resamples);
    for _ in 0..policy.bootstrap_resamples {
        let mut sum = 0.0;
        let mut sampled = 0;
        while sampled < pairs.len() {
            let start = index(&mut state, gains.len());
            let block = policy.resampling_block_length.min(pairs.len() - sampled);
            for offset in 0..block {
                sum += gains[(start + offset) % gains.len()];
            }
            sampled += block;
        }
        resamples.push(sum / pairs.len() as f64);
    }
    resamples.sort_by(f64::total_cmp);
    let tail_probability =
        f64::from(10000 - policy.confidence_basis_points) / 20000.0 / family_size as f64;
    let comparison_confidence_basis_points =
        10000.0 - f64::from(10000 - policy.confidence_basis_points) / family_size as f64;
    let lower_count = (tail_probability * resamples.len() as f64).ceil().max(1.0) as usize;
    let upper_count = ((1.0 - tail_probability) * resamples.len() as f64)
        .ceil()
        .max(1.0) as usize;
    let lower = lower_count - 1;
    let upper = upper_count.saturating_sub(1).min(resamples.len() - 1);
    let comparison_tail_resamples = lower_count.min(resamples.len() - upper);
    let interval = [resamples[lower], resamples[upper]];
    let baseline = distribution(
        pairs.iter().map(|p| p.baseline_ns).collect(),
        resampling_blocks,
    );
    let candidate = distribution(
        pairs.iter().map(|p| p.candidate_ns).collect(),
        resampling_blocks,
    );
    let p95_ok = baseline
        .p95_ns
        .zip(candidate.p95_ns)
        .map(|(a, b)| b.saturating_sub(a) <= policy.p95_regression_budget_ns);
    let p99_ok = match policy.p99_regression_budget_ns {
        None => Some(true),
        Some(budget) => baseline
            .p99_ns
            .zip(candidate.p99_ns)
            .map(|(a, b)| b.saturating_sub(a) <= budget),
    };
    let decision = if comparison_tail_resamples < MIN_COMPARISON_TAIL_RESAMPLES {
        "inadequate_comparison_resolution"
    } else if p95_ok.is_none() || p99_ok.is_none() {
        "insufficient_tail_evidence"
    } else if p95_ok == Some(false) || p99_ok == Some(false) {
        "tail_regression"
    } else if interval[0] > policy.improvement_margin_ns as f64 {
        "supported_latency_gain"
    } else {
        "inconclusive_effect"
    };
    Ok(Statistics {
        paired_runs: pairs.len(),
        resampling_block_length: policy.resampling_block_length,
        resampling_blocks,
        comparison_family_size: family_size,
        comparison_confidence_basis_points,
        comparison_tail_resamples,
        baseline,
        candidate,
        mean_gain_ns: mean,
        mean_gain_fraction: mean / baseline_mean,
        gain_interval_ns: interval,
        confidence_basis_points: policy.confidence_basis_points,
        bootstrap_resamples: policy.bootstrap_resamples,
        seed: policy.seed,
        decision: decision.into(),
    })
}

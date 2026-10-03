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
    pub p95_memory_regression_budget_bytes: Option<u64>,
    pub p95_copied_bytes_regression_budget: Option<u64>,
    pub p95_energy_regression_budget_uj: Option<u64>,
    /// A prospectively declared, pilot-based power plan. Missing plans cannot support a gain.
    #[serde(default)]
    pub prospective_power: Option<ProspectivePowerPlan>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProspectivePowerPlan {
    /// Digest of the separately authenticated, ordered pilot evidence.
    pub pilot_digest: String,
    /// Expected mean paired latency gain declared before the measured run.
    pub expected_mean_gain_ns: u64,
    pub target_power_basis_points: u32,
    /// Planned effective resampling blocks, not raw requests or within-run observations.
    pub planned_effective_blocks: usize,
}
impl StatisticalPolicy {
    pub fn validate(&self) -> Result<(), String> {
        if !(MIN_RESAMPLING_BLOCKS..=MAX_PAIRS).contains(&self.min_resampling_blocks)
            || self.resampling_block_length == 0
            || self.resampling_block_length > self.min_resampling_blocks / 2
            || !(MIN_RESAMPLES..=MAX_RESAMPLES).contains(&self.bootstrap_resamples)
            || !(9000..=9900).contains(&self.confidence_basis_points)
            || self.prospective_power.as_ref().is_some_and(|plan| {
                !crate::digest_valid(&plan.pilot_digest)
                    || plan.expected_mean_gain_ns <= self.improvement_margin_ns
                    || !(8000..=9900).contains(&plan.target_power_basis_points)
                    || !(MIN_RESAMPLING_BLOCKS..=MAX_PAIRS).contains(&plan.planned_effective_blocks)
            })
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
pub struct ResourceDistribution {
    pub median: u64,
    pub p95: Option<u64>,
    pub p99: Option<u64>,
}
#[derive(Clone, Debug, Serialize)]
pub struct ResourceComparison {
    pub baseline: ResourceDistribution,
    pub candidate: ResourceDistribution,
}
#[derive(Clone, Debug, Serialize)]
pub struct Statistics {
    pub paired_runs: usize,
    pub resampling_block_length: usize,
    pub resampling_blocks: usize,
    pub comparison_family_size: usize,
    pub comparison_confidence_basis_points: f64,
    pub comparison_tail_resamples: usize,
    pub dependence: DependenceDiagnostic,
    pub prospective_power: Option<ProspectivePowerAssessment>,
    pub baseline: Distribution,
    pub candidate: Distribution,
    pub memory_bytes: Option<ResourceComparison>,
    pub copied_bytes: Option<ResourceComparison>,
    pub energy_uj: Option<ResourceComparison>,
    pub mean_gain_ns: f64,
    pub mean_gain_fraction: f64,
    pub gain_interval_ns: [f64; 2],
    pub confidence_basis_points: u32,
    pub bootstrap_resamples: usize,
    pub seed: u64,
    pub decision: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ProspectivePowerAssessment {
    pub pilot_pairs: usize,
    pub pilot_effective_blocks: usize,
    pub pilot_block_stddev_ns: f64,
    pub expected_mean_gain_ns: u64,
    pub improvement_margin_ns: u64,
    pub family_adjusted_lower_critical_percentile_basis_points: f64,
    pub planned_effective_blocks: usize,
    pub actual_effective_blocks: usize,
    pub required_effective_blocks: Option<usize>,
    pub planned_power_basis_points: f64,
    pub target_power_basis_points: u32,
    pub adequate: bool,
    /// Large-sample normal approximation for the mean of independent pilot blocks.
    pub method: &'static str,
}

#[derive(Clone, Debug, Serialize)]
pub struct DependenceDiagnostic {
    /// Lag-one autocorrelation of ordered whole-run latency gains.
    pub lag_one_autocorrelation: Option<f64>,
    /// Last lag retained by the positive-pair truncation, capped at 100.
    pub positive_sequence_cutoff_lag: Option<usize>,
    /// Initial-positive-sequence estimate from ordered paired gains; diagnostic only.
    pub estimated_effective_pairs: Option<f64>,
    pub maximum_lag_examined: usize,
}

fn dependence_diagnostic(gains: &[f64]) -> DependenceDiagnostic {
    let maximum_lag_examined = (gains.len() / 2).min(100);
    let mean = gains.iter().sum::<f64>() / gains.len() as f64;
    let centered: Vec<_> = gains.iter().map(|value| value - mean).collect();
    let denominator = centered.iter().map(|value| value * value).sum::<f64>();
    if denominator == 0.0 || maximum_lag_examined < 2 {
        return DependenceDiagnostic {
            lag_one_autocorrelation: None,
            positive_sequence_cutoff_lag: None,
            estimated_effective_pairs: None,
            maximum_lag_examined,
        };
    }

    let autocorrelation = |lag: usize| {
        centered[lag..]
            .iter()
            .zip(&centered[..centered.len() - lag])
            .map(|(later, earlier)| later * earlier)
            .sum::<f64>()
            / denominator
    };
    let lag_one_autocorrelation = autocorrelation(1);
    let mut integrated_correlation = 0.0;
    let mut positive_sequence_cutoff_lag = None;
    let mut lag = 1;
    while lag < maximum_lag_examined {
        let pair_sum = autocorrelation(lag) + autocorrelation(lag + 1);
        if pair_sum <= 0.0 {
            break;
        }
        integrated_correlation += pair_sum;
        positive_sequence_cutoff_lag = Some(lag + 1);
        lag += 2;
    }
    let integrated_time = (1.0 + 2.0 * integrated_correlation).max(1.0);
    DependenceDiagnostic {
        lag_one_autocorrelation: Some(lag_one_autocorrelation),
        positive_sequence_cutoff_lag,
        estimated_effective_pairs: Some((gains.len() as f64 / integrated_time).max(1.0)),
        maximum_lag_examined,
    }
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

fn resource_distribution(mut values: Vec<u64>, effective_units: usize) -> ResourceDistribution {
    values.sort_unstable();
    let rank = |percent: usize| values[(percent * values.len()).div_ceil(100) - 1];
    ResourceDistribution {
        median: rank(50),
        p95: (effective_units >= P95_TAIL_MINIMUM).then(|| rank(95)),
        p99: (effective_units >= P99_TAIL_MINIMUM).then(|| rank(99)),
    }
}

fn resource_comparison(
    pairs: &[Pair],
    baseline: impl Fn(&Pair) -> Option<u64>,
    candidate: impl Fn(&Pair) -> Option<u64>,
    effective_units: usize,
) -> Option<ResourceComparison> {
    let baseline_values: Option<Vec<_>> = pairs.iter().map(baseline).collect();
    let candidate_values: Option<Vec<_>> = pairs.iter().map(candidate).collect();
    Some(ResourceComparison {
        baseline: resource_distribution(baseline_values?, effective_units),
        candidate: resource_distribution(candidate_values?, effective_units),
    })
}

fn within_budget(
    comparison: Option<&ResourceComparison>,
    budget: Option<u64>,
    effective_units: usize,
) -> Option<bool> {
    let Some(budget) = budget else {
        return Some(true);
    };
    let comparison = comparison?;
    if effective_units < P95_TAIL_MINIMUM {
        return None;
    }
    Some(
        comparison
            .candidate
            .p95?
            .saturating_sub(comparison.baseline.p95?)
            <= budget,
    )
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
    let memory_bytes = resource_comparison(
        pairs,
        |p| p.baseline_memory_bytes,
        |p| p.candidate_memory_bytes,
        resampling_blocks,
    );
    let copied_bytes = resource_comparison(
        pairs,
        |p| p.baseline_copied_bytes,
        |p| p.candidate_copied_bytes,
        resampling_blocks,
    );
    let energy_uj = resource_comparison(
        pairs,
        |p| p.baseline_energy_uj,
        |p| p.candidate_energy_uj,
        resampling_blocks,
    );
    let memory_ok = within_budget(
        memory_bytes.as_ref(),
        policy.p95_memory_regression_budget_bytes,
        resampling_blocks,
    );
    let copied_bytes_ok = within_budget(
        copied_bytes.as_ref(),
        policy.p95_copied_bytes_regression_budget,
        resampling_blocks,
    );
    let energy_ok = within_budget(
        energy_uj.as_ref(),
        policy.p95_energy_regression_budget_uj,
        resampling_blocks,
    );
    let decision = if comparison_tail_resamples < MIN_COMPARISON_TAIL_RESAMPLES {
        "inadequate_comparison_resolution"
    } else if p95_ok.is_none() || p99_ok.is_none() {
        "insufficient_tail_evidence"
    } else if memory_ok.is_none() || copied_bytes_ok.is_none() || energy_ok.is_none() {
        "insufficient_resource_evidence"
    } else if p95_ok == Some(false) || p99_ok == Some(false) {
        "tail_regression"
    } else if memory_ok == Some(false) || copied_bytes_ok == Some(false) || energy_ok == Some(false)
    {
        "resource_regression"
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
        dependence: dependence_diagnostic(&gains),
        prospective_power: None,
        baseline,
        candidate,
        memory_bytes,
        copied_bytes,
        energy_uj,
        mean_gain_ns: mean,
        mean_gain_fraction: mean / baseline_mean,
        gain_interval_ns: interval,
        confidence_basis_points: policy.confidence_basis_points,
        bootstrap_resamples: policy.bootstrap_resamples,
        seed: policy.seed,
        decision: decision.into(),
    })
}

/// Applies the declared normal-theory prospective power plan to authenticated paired-pilot gains.
/// The pilot must be independent of the measured pairs and must retain its order.
pub fn analyze_family_with_power(
    pairs: &[Pair],
    policy: &StatisticalPolicy,
    family_size: usize,
    pilot_gains_ns: Option<&[i64]>,
) -> Result<Statistics, String> {
    let mut result = analyze_family(pairs, policy, family_size)?;
    let Some(plan) = policy.prospective_power.as_ref() else {
        if result.decision == "supported_latency_gain" {
            result.decision = "missing_prospective_power".into();
        }
        return Ok(result);
    };
    let Some(pilot_gains_ns) = pilot_gains_ns else {
        if result.decision == "supported_latency_gain" {
            result.decision = "missing_power_pilot".into();
        }
        return Ok(result);
    };
    let block_length = policy.resampling_block_length;
    if pilot_gains_ns.len() < MIN_RESAMPLING_BLOCKS * block_length
        || pilot_gains_ns.len() > MAX_PAIRS
        || !pilot_gains_ns.len().is_multiple_of(block_length)
    {
        return Err("power pilot must contain at least 30 complete ordered blocks".into());
    }
    let pilot_blocks: Vec<f64> = pilot_gains_ns
        .chunks_exact(block_length)
        .map(|chunk| chunk.iter().map(|gain| *gain as f64).sum::<f64>() / block_length as f64)
        .collect();
    if dependence_diagnostic(&pilot_blocks)
        .estimated_effective_pairs
        .is_some_and(|effective| effective < MIN_RESAMPLING_BLOCKS as f64)
    {
        return Err("power pilot has fewer than 30 estimated effective blocks".into());
    }
    let pilot_mean = pilot_blocks.iter().sum::<f64>() / pilot_blocks.len() as f64;
    let sum_squares = pilot_blocks
        .iter()
        .map(|gain| (gain - pilot_mean).powi(2))
        .sum::<f64>();
    let pilot_block_stddev_ns = (sum_squares / (pilot_blocks.len() - 1) as f64).sqrt();
    if pilot_block_stddev_ns == 0.0 || !pilot_block_stddev_ns.is_finite() {
        return Err("power pilot must establish non-zero finite block variation".into());
    }

    let alpha = 1.0 - f64::from(policy.confidence_basis_points) / 10000.0;
    let critical_probability = 1.0 - alpha / (2.0 * family_size as f64);
    let critical_z = inverse_standard_normal(critical_probability);
    let target_probability = f64::from(plan.target_power_basis_points) / 10000.0;
    let target_z = inverse_standard_normal(target_probability);
    let effect = (plan.expected_mean_gain_ns - policy.improvement_margin_ns) as f64;
    let required = ((critical_z + target_z) * pilot_block_stddev_ns / effect).powi(2);
    let required_effective_blocks = required
        .is_finite()
        .then(|| required.ceil().max(1.0))
        .and_then(|value| (value <= usize::MAX as f64).then_some(value as usize));
    let noncentrality =
        effect * (plan.planned_effective_blocks as f64).sqrt() / pilot_block_stddev_ns;
    let planned_power_basis_points =
        (normal_cdf(noncentrality - critical_z) * 10000.0).clamp(0.0, 10000.0);
    let adequate = required_effective_blocks
        .is_some_and(|required| plan.planned_effective_blocks >= required)
        && result.resampling_blocks == plan.planned_effective_blocks
        && pairs.len().is_multiple_of(block_length)
        && planned_power_basis_points >= f64::from(plan.target_power_basis_points);
    let assessment = ProspectivePowerAssessment {
        pilot_pairs: pilot_gains_ns.len(),
        pilot_effective_blocks: pilot_blocks.len(),
        pilot_block_stddev_ns,
        expected_mean_gain_ns: plan.expected_mean_gain_ns,
        improvement_margin_ns: policy.improvement_margin_ns,
        family_adjusted_lower_critical_percentile_basis_points: critical_probability * 10000.0,
        planned_effective_blocks: plan.planned_effective_blocks,
        actual_effective_blocks: result.resampling_blocks,
        required_effective_blocks,
        planned_power_basis_points,
        target_power_basis_points: plan.target_power_basis_points,
        adequate,
        method: "paired_block_mean_normal_approximation",
    };
    if !adequate && result.decision == "supported_latency_gain" {
        result.decision = "insufficient_prospective_power".into();
    }
    result.prospective_power = Some(assessment);
    Ok(result)
}

fn inverse_standard_normal(p: f64) -> f64 {
    // Peter Acklam's rational approximation; the validated inputs stay in (0, 1).
    const A: [f64; 6] = [
        -3.969683028665376e1,
        2.209460984245205e2,
        -2.759285104469687e2,
        1.38357751867269e2,
        -3.066479806614716e1,
        2.506628277459239,
    ];
    const B: [f64; 5] = [
        -5.447609879822406e1,
        1.615858368580409e2,
        -1.556989798598866e2,
        6.680131188771972e1,
        -1.328068155288572e1,
    ];
    const C: [f64; 6] = [
        -7.784894002430293e-3,
        -3.223964580411365e-1,
        -2.400758277161838,
        -2.549732539343734,
        4.374664141464968,
        2.938163982698783,
    ];
    const D: [f64; 4] = [
        7.784695709041462e-3,
        3.224671290700398e-1,
        2.445134137142996,
        3.754408661907416,
    ];
    const LOW: f64 = 0.02425;
    if p < LOW {
        let q = (-2.0 * p.ln()).sqrt();
        (((((C[0] * q + C[1]) * q + C[2]) * q + C[3]) * q + C[4]) * q + C[5])
            / ((((D[0] * q + D[1]) * q + D[2]) * q + D[3]) * q + 1.0)
    } else if p > 1.0 - LOW {
        -inverse_standard_normal(1.0 - p)
    } else {
        let q = p - 0.5;
        let r = q * q;
        (((((A[0] * r + A[1]) * r + A[2]) * r + A[3]) * r + A[4]) * r + A[5]) * q
            / (((((B[0] * r + B[1]) * r + B[2]) * r + B[3]) * r + B[4]) * r + 1.0)
    }
}

fn normal_cdf(value: f64) -> f64 {
    // Abramowitz-Stegun 7.1.26 approximation, sufficient for this planning diagnostic.
    let x = value.abs();
    let t = 1.0 / (1.0 + 0.2316419 * x);
    let density = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let tail = density
        * t
        * (0.319381530
            + t * (-0.356563782 + t * (1.781477937 + t * (-1.821255978 + t * 1.330274429))));
    if value >= 0.0 { 1.0 - tail } else { tail }
}

#[cfg(test)]
mod dependence_tests {
    use super::{dependence_diagnostic, inverse_standard_normal, normal_cdf};

    #[test]
    fn normal_approximations_cover_the_power_plan_quantiles() {
        assert!((inverse_standard_normal(0.975) - 1.9599639845).abs() < 1e-7);
        assert!((normal_cdf(1.9599639845) - 0.975).abs() < 2e-7);
        assert!((normal_cdf(-1.9599639845) - 0.025).abs() < 2e-7);
    }

    #[test]
    fn ordered_gain_diagnostic_exposes_serial_dependence_without_claiming_power() {
        let trending: Vec<_> = (0..100).map(f64::from).collect();
        let correlated = dependence_diagnostic(&trending);
        assert!(correlated.lag_one_autocorrelation.unwrap() > 0.9);
        assert!(correlated.estimated_effective_pairs.unwrap() < 10.0);
        assert_eq!(correlated.maximum_lag_examined, 50);

        let alternating: Vec<_> = (0..100)
            .map(|i| if i % 2 == 0 { -1.0 } else { 1.0 })
            .collect();
        let anti_correlated = dependence_diagnostic(&alternating);
        assert!(anti_correlated.lag_one_autocorrelation.unwrap() < -0.9);
        assert!(anti_correlated.estimated_effective_pairs.unwrap() <= 100.0);

        let constant = dependence_diagnostic(&vec![7.0; 100]);
        assert!(constant.lag_one_autocorrelation.is_none());
        assert!(constant.estimated_effective_pairs.is_none());
    }
}

//! Preregistered descriptive boot-pair analysis; never changes admission or samples.
use super::*;
fn descriptive(values: &Value) -> Value {
    let Some(values) = values.as_array() else {
        return json!({"state":"UNAVAILABLE","reason":"samples absent"});
    };
    let mut n = 0u64;
    let mut mean = 0.0;
    let mut m2 = 0.0;
    let mut integers = Vec::new();
    for value in values {
        let Some(value) = value.as_u64() else {
            return json!({"state":"UNAVAILABLE","reason":"invalid raw sample"});
        };
        integers.push(value);
        n += 1;
        let delta = value as f64 - mean;
        mean += delta / n as f64;
        m2 += delta * (value as f64 - mean);
    }
    let mut out = summary(&integers);
    out["mean_ticks"] = if n > 0 { json!(mean) } else { Value::Null };
    out["sample_variance_ticks_squared"] = if n > 1 {
        json!(m2 / (n - 1) as f64)
    } else {
        Value::Null
    };
    out["sample_stddev_ticks"] = if n > 1 {
        json!((m2 / (n - 1) as f64).sqrt())
    } else {
        Value::Null
    };
    out["dispersion_scope"] = json!(
        "descriptive within-boot dispersion; correlated observations, not independent uncertainty"
    );
    out
}
fn effect(off: Option<f64>, on: Option<f64>) -> Value {
    match off.zip(on) {
        Some((a, b)) if a.is_finite() && b.is_finite() => {
            json!({"difference_on_minus_off":b-a,"log_ratio_on_over_off":if a>0.0&&b>0.0 {Some((b/a).ln())}else{None},"ratio_reason":if a>0.0&&b>0.0 {"defined"}else{"nonpositive denominator or numerator; no substituted observation"}})
        }
        _ => {
            json!({"difference_on_minus_off":null,"log_ratio_on_over_off":null,"ratio_reason":"missing or invalid boot statistic"})
        }
    }
}
fn interval(values: &[Option<f64>], interpretation_valid: bool, ratio: bool) -> Value {
    if values.len() != 12 || values.iter().any(|v| v.is_none_or(|v| !v.is_finite())) {
        return json!({"state":"UNAVAILABLE","reason":"fixed twelve paired observations not all defined; no replacement or reduced-sample interval","paired_values":values});
    }
    let mut sorted: Vec<f64> = values.iter().map(|v| v.unwrap()).collect();
    sorted.sort_by(f64::total_cmp);
    let convert = |v: f64| if ratio { v.exp() } else { v };
    json!({"state":if interpretation_valid{"AVAILABLE"}else{"INCONCLUSIVE"},"paired_values":values,"median":convert((sorted[5]+sorted[6])/2.0),"lower":convert(sorted[2]),"upper":convert(sorted[9]),"nominal_conditional_coverage":0.96142578125_f64,"interval":"third to tenth sorted paired values, endpoints exponentiated for log ratios","assumptions":"IID continuous boot-pair observations; host drift can invalidate interpretation; marginal not simultaneous coverage; no superiority or precision guarantee","interpretation":if interpretation_valid{"matched observed outcome vectors; recorder envelope comparison, not exclusive CPU recorder cost"}else{"outcome vectors differ or pair validity fails; numerical summaries remain descriptive and cannot isolate recorder cost"}})
}
fn saturation(result: &Value) -> Value {
    let mut groups = [Vec::new(), Vec::new(), Vec::new()];
    if let Some(records) = result["records"].as_array() {
        for r in records.iter().filter(|r| r["phase"] == 1) {
            if let (Some(id), Some(wall)) = (r["request_id"].as_u64(), r["wall_ticks"].as_u64()) {
                let index = (id & 0xffff_ffff).checked_sub(1);
                if let Some(i) = index {
                    groups[(i % 3) as usize].push(wall);
                }
            }
        }
    }
    json!({"A_admitted":{"wall_ticks":groups[0],"summary":descriptive(&json!(groups[0])),"scope":"includes nested B refusal/probes and feedback release before A Collect"},"B_refused":{"wall_ticks":groups[1],"summary":descriptive(&json!(groups[1])),"scope":"actual Exhausted refusal envelope; zero useful successes"},"C_recovered":{"wall_ticks":groups[2],"summary":descriptive(&json!(groups[2])),"scope":"fresh successful request after A drained"}})
}
pub(super) fn write(root: &Path, campaign: &Value) -> Result<()> {
    let attempts = campaign["attempts"]
        .as_array()
        .ok_or("analysis attempts absent")?;
    let mut cells = Vec::new();
    for case in 0..12 {
        for profile in ["dev", "prod"] {
            let mut pairs = Vec::new();
            let mut interpretations = true;
            for pair in 0..12 {
                let rows: Vec<_> = attempts
                    .iter()
                    .filter(|a| a["case"] == case && a["profile"] == profile && a["pair"] == pair)
                    .collect();
                let valid = passport::fixed_pair(&rows, pair).ok().filter(|(a, b)| {
                    passport::revalidated_attempt(a) && passport::revalidated_attempt(b)
                });
                let Some((off, on)) = valid else {
                    interpretations = false;
                    pairs.push(json!({"pair":pair,"order":if pair%2==0{"OFF/ON"}else{"ON/OFF"},"state":"UNAVAILABLE","reason":"missing, failed, duplicated, relabeled or corrupt fixed pair","latency":effect(None,None),"throughput":effect(None,None)}));
                    continue;
                };
                let (a, b) = (&off["result"], &on["result"]);
                let outcomes = passport::outcomes_match(a, b)?;
                interpretations &= outcomes;
                let latency = effect(
                    a["successful_summary"]["p50"].as_f64(),
                    b["successful_summary"]["p50"].as_f64(),
                );
                let throughput = effect(
                    a["throughput"]["successful_operations_per_second"].as_f64(),
                    b["throughput"]["successful_operations_per_second"].as_f64(),
                );
                pairs.push(json!({"pair":pair,"order":if pair%2==0{"OFF/ON"}else{"ON/OFF"},"state":if outcomes{"AVAILABLE"}else{"INCONCLUSIVE"},"outcomes_match":outcomes,"latency":latency,"throughput":throughput,"off":{"summary":descriptive(&a["successful_wall_ticks"]),"throughput":a["throughput"],"measured_successes":a["measured_successes"],"total_exhausted":a["exhausted"],"userspace_overlap":a["userspace_overlap"],"saturation":if case==9{saturation(a)}else{Value::Null}},"on":{"summary":descriptive(&b["successful_wall_ticks"]),"throughput":b["throughput"],"measured_successes":b["measured_successes"],"total_exhausted":b["exhausted"],"userspace_overlap":b["userspace_overlap"],"saturation":if case==9{saturation(b)}else{Value::Null}}}));
            }
            let mut effects = serde_json::Map::new();
            for metric in ["latency", "throughput"] {
                let differences: Vec<_> = pairs
                    .iter()
                    .map(|p| p[metric]["difference_on_minus_off"].as_f64())
                    .collect();
                let ratios: Vec<_> = pairs
                    .iter()
                    .map(|p| p[metric]["log_ratio_on_over_off"].as_f64())
                    .collect();
                effects.insert(metric.into(),json!({"paired_differences":interval(&differences,interpretations,false),"paired_ratios":interval(&ratios,interpretations,true)}));
            }
            cells.push(json!({"case":case,"profile":profile,"pairs":pairs,"effects":effects,"tail_adequacy":"INCONCLUSIVE; per-boot p95/p99 remain descriptive, never averaged as pooled quantiles"}));
        }
    }
    write_json(
        root.join("paired-analysis.json"),
        &json!({"schema_version":1,"analysis_version":"1","record_eligible":false,"scope":"preregistered twelve boot-pair effects; no admission changes, pooled-tail claim or performance allowance","campaign_functional_status":campaign["functional_status"],"cells":cells}),
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dispersion_is_descriptive_and_undefined_for_one_sample() {
        let x = descriptive(&json!([1, 3]));
        assert_eq!(x["mean_ticks"], 2.0);
        assert_eq!(x["sample_variance_ticks_squared"], 2.0);
        assert!(descriptive(&json!([5]))["sample_stddev_ticks"].is_null());
    }
    #[test]
    fn order_interval_keeps_twelve_pairs_and_rejects_missing() {
        let v: Vec<_> = (1..=12).map(|v| Some(v as f64)).collect();
        let x = interval(&v, true, false);
        assert_eq!(x["lower"], 3.0);
        assert_eq!(x["upper"], 10.0);
        assert_eq!(x["median"], 6.5);
        assert_eq!(interval(&v, false, false)["state"], "INCONCLUSIVE");
        let mut missing = v;
        missing[3] = None;
        assert_eq!(interval(&missing, true, false)["state"], "UNAVAILABLE");
        assert!(effect(Some(0.0), Some(1.0))["log_ratio_on_over_off"].is_null());
    }
    #[test]
    fn saturation_groups_do_not_pool_admit_refusal_and_recovery() {
        let result = json!({"records":[{"phase":0,"request_id":1,"wall_ticks":999},{"phase":1,"request_id":4,"wall_ticks":50},{"phase":1,"request_id":5,"wall_ticks":2},{"phase":1,"request_id":6,"wall_ticks":7}]});
        let s = saturation(&result);
        assert_eq!(s["A_admitted"]["wall_ticks"], json!([50]));
        assert_eq!(s["B_refused"]["wall_ticks"], json!([2]));
        assert_eq!(s["C_recovered"]["wall_ticks"], json!([7]));
    }
}

use migration_advisor::routing_evidence::inspect;
use serde_json::{Value, json};

fn evidence() -> Value {
    let mut words: Vec<u64> = vec![
        0x4b56_5232,
        0,
        0,
        1,
        24,
        123,
        2,
        1_000_000,
        0,
        0,
        1,
        1,
        2,
        3,
        4,
    ];
    words.extend([0x4245_4e43, 0, 128]);
    words.extend([10; 16]);
    words.extend([10; 128]);
    words.push(0);
    let counters = [144, 0, 144, 0, 0, 0, 0, 0, 0];
    words.extend(counters);
    words.push(0x444f_4e45);
    let count = words.len();
    json!({"schema_version":1,"runs":[{"profile":"fixture-dev","kernel_build":{"sha256":"a".repeat(64)},"user_artifact":{"elf_sha256":"b".repeat(64)},
        "run":{"elf_sha256":"a".repeat(64),"accelerator":"TCG"},
        "events":[{"event":"el0","status":"pass","reclaimed":true,"processes":1},
            {"event":"user-report","id":0,"offset":0,"words":words},
            {"event":"user-result","id":0,"state":2,"exit":1,"fault":0,"length":count},
            {"event":"boot","status":"pass","el":1,"secondary_shutdown_verified":true}],
        "consumers":[{"id":0,"cpu":0,"route":0,"generation":1,"conformance_checks":24,"oracle_sum":123,"frequency":1_000_000,"profile_digest_words":[1,2,3,4],
            "benchmarks":[{"route":0,"warmup_samples":vec![10;16],"samples":vec![10;128],"counters":counters,"observed_preemptions":0}]}]}]})
}

#[test]
fn authentic_structure_preserves_raw_observations_without_inventing_ab_pairs() {
    let report = inspect(&serde_json::to_vec(&evidence()).unwrap()).unwrap();
    assert_eq!(
        report.runs[0].observations[0].measured_routes[0].median_wall_ns,
        10000
    );
    assert_eq!(
        report.runs[0].observations[0].measured_routes[0].native_admissions,
        144
    );
    assert!(!report.independent_paired_runs);
    assert!(!report.provenance_verified);
    assert!(!report.automatic_replacement);
    assert!(report.preferred_migration.is_none());
}

#[test]
fn damaged_missing_or_inconsistent_os_evidence_is_rejected() {
    let mutations: Vec<fn(&mut Value)> = vec![
        |v| v["runs"][0]["consumers"][0]["benchmarks"][0]["samples"][0] = 11.into(),
        |v| v["runs"][0]["consumers"][0]["benchmarks"][0]["counters"][0] = 143.into(),
        |v| v["runs"][0]["consumers"][0]["frequency"] = 0.into(),
        |v| v["runs"][0]["events"][1]["offset"] = 1.into(),
        |v| v["runs"][0]["events"][2]["exit"] = 0.into(),
        |v| {
            v["runs"][0]["events"].as_array_mut().unwrap().pop();
        },
        |v| v["runs"][0]["run"]["elf_sha256"] = "c".repeat(64).into(),
    ];
    for mutate in mutations {
        let mut data = evidence();
        mutate(&mut data);
        assert!(inspect(&serde_json::to_vec(&data).unwrap()).is_err());
    }
}

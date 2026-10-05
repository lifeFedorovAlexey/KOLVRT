#![forbid(unsafe_code)]

use repository_checks::{CheckResult, cost_l, database, documents, knowledge, report};
use std::{env, path::PathBuf};

fn run() -> CheckResult<()> {
    let args: Vec<_> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("check");
    let mut root = env::current_dir().map_err(|e| e.to_string())?;
    while !root.join("schemas/research-case.schema.json").is_file() {
        if !root.pop() {
            return Err("run from the KOLVRT repository".into());
        }
    }
    match command {
        "arena" => return repository_checks::arena::cli(&root, &args[1..]),
        "cost-l" => return repository_checks::cost_l_queries::cli(&root, &args[1..]),
        "docs" => return knowledge::cli(&root, &args[1..]),
        "check" | "validate" | "check-docs" | "check-translations" if args.len() <= 1 => {
            if matches!(command,"check" | "validate") { database::validate(&root, &root.join("research/cases"), repository_checks::MINIMUM_RESEARCH_CASES)?; }
            if matches!(command,"check" | "validate") { cost_l::validate(&root, &root.join("research/cost-l"))?; }
            if matches!(command,"check" | "check-docs") { documents::check_docs(&root)?; }
            if matches!(command,"check" | "check-translations") { documents::check_translations(&root)?; }
            if command == "check" { repository_checks::arena::check(&root)?; report::report(&root, true)?; knowledge::generate(&root, true)?; knowledge::pilot(&root, true)?; }
            println!("{command}: passed. Structural checks do not establish historical truth or kernel behavior.");
        }
        "check-cost-l" if args.len() == 1 || (args.len() == 3 && args[1] == "--directory") => {
            let directory = if args.len() == 3 { PathBuf::from(&args[2]) } else { root.join("research/cost-l") };
            let records = cost_l::validate(&root, &directory)?;
            println!("COST-L: {} records validated; no runtime support or authority established.", records.len());
        }
        "validate" if args.len() == 3 && args[1] == "--directory" => {
            database::validate(&root, &PathBuf::from(&args[2]), repository_checks::MINIMUM_RESEARCH_CASES)?;
            println!("Research data validated.");
        }
        "report" if args.len() == 1 || (args.len() == 2 && args[1] == "--check") => {
            report::report(&root, args.len() == 2)?;
            println!("Case indexes {}.", if args.len() == 2 {"are current"} else {"regenerated; review and record changed translation pairs"});
        }
        "record-translation" if args.len() == 3 => {
            documents::record_translation(&root, &args[1], &args[2])?;
            println!("Recorded reviewed pair: {}/{}. Meaning must be reviewed by a person.", args[1], args[2]);
        }
        _ => return Err("usage: repository-checks [cost-l COMMAND [OPTIONS] | check | validate [--directory PATH] | check-cost-l [--directory PATH] | check-docs | check-translations | report [--check] | record-translation LOCALE PATH]".into())
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

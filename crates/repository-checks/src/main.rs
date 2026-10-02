#![forbid(unsafe_code)]

use repository_checks::{CheckResult, database, documents, report};
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
        "check" | "validate" | "check-docs" | "check-translations" if args.len() <= 1 => {
            if matches!(command,"check" | "validate") { database::validate(&root, &root.join("research/cases"), 30)?; }
            if matches!(command,"check" | "check-docs") { documents::check_docs(&root)?; }
            if matches!(command,"check" | "check-translations") { documents::check_translations(&root)?; }
            if command == "check" { report::report(&root, true)?; }
            println!("{command}: passed. Structural checks do not establish historical truth or kernel behavior.");
        }
        "validate" if args.len() == 3 && args[1] == "--directory" => {
            database::validate(&root, &PathBuf::from(&args[2]), 30)?;
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
        _ => return Err("usage: repository-checks [check | validate [--directory PATH] | check-docs | check-translations | report [--check] | record-translation LOCALE PATH]".into())
    }
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

//! `pintsg` — command-line front end.
//!
//!     pintsg invoice.xml            # human-readable report, exit 1 if it fails
//!     pintsg --json invoice.xml     # machine-readable report on stdout
//!     cat invoice.xml | pintsg -    # read from stdin
//!
//! Exit code is 0 when the document conforms (no errors), 1 otherwise — so it
//! drops straight into a CI pipeline or a pre-send gate.

use std::io::Read;
use std::process::ExitCode;

use pint_sg_validator::{validate, Severity};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    let path = args.iter().find(|a| !a.starts_with('-'));

    let xml = match path.map(String::as_str) {
        None => {
            eprintln!("usage: pintsg [--json] <file.xml | ->");
            return ExitCode::from(2);
        }
        Some("-") => {
            let mut s = String::new();
            if std::io::stdin().read_to_string(&mut s).is_err() {
                eprintln!("error: could not read stdin");
                return ExitCode::from(2);
            }
            s
        }
        Some(p) => match std::fs::read_to_string(p) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: cannot read {p}: {e}");
                return ExitCode::from(2);
            }
        },
    };

    let report = validate(&xml);

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        if report.findings.is_empty() {
            println!("✓ conforms — no findings");
        }
        for f in &report.findings {
            let tag = match f.severity {
                Severity::Error => "ERROR  ",
                Severity::Warning => "WARN   ",
                Severity::Info => "INFO   ",
            };
            println!("{tag} [{}] {}", f.rule, f.message);
        }
        println!(
            "\n{} — {} error(s), {} warning(s)",
            if report.conforms { "PASS" } else { "FAIL" },
            report.errors,
            report.warnings
        );
    }

    if report.conforms { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

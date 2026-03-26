//! `txtcode bench` — micro-benchmark a Txt-code program.

use crate::builder::{BuildConfig, Builder};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::validator::Validator;
use std::fs;
use std::path::PathBuf;

pub fn benchmark_file(
    file: &PathBuf,
    runs: usize,
    warmup: usize,
    save: Option<&PathBuf>,
    compare: Option<&PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    if !file.exists() {
        return Err(format!("File '{}' not found", file.display()).into());
    }
    if file.is_dir() {
        return Err(format!("'{}' is a directory", file.display()).into());
    }

    let prev: Option<(f64, f64, f64, f64)> = if let Some(cmp_path) = compare {
        match fs::read_to_string(cmp_path) {
            Ok(data) => parse_bench_json(&data),
            Err(e) => {
                eprintln!(
                    "Warning: could not read compare file '{}': {}",
                    cmp_path.display(),
                    e
                );
                None
            }
        }
    } else {
        None
    };

    let source = fs::read_to_string(file)?;
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().map_err(|e| format!("Lex error: {}", e))?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse().map_err(|e| format!("Parse error: {}", e))?;
    Validator::validate_program(&program)
        .map_err(|e| format!("Validation error: {}", e))?;

    let bench_config = BuildConfig::default();

    println!("Benchmarking: {}", file.display());
    println!("  Warmup: {} run(s), measured: {} run(s)\n", warmup, runs);

    for _ in 0..warmup {
        let mut vm = Builder::create_vm(&bench_config);
        vm.interpret(&program)
            .map_err(|e| format!("Runtime error: {}", e))?;
    }

    let mut timings: Vec<f64> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = std::time::Instant::now();
        let mut vm = Builder::create_vm(&bench_config);
        vm.interpret(&program)
            .map_err(|e| format!("Runtime error: {}", e))?;
        timings.push(start.elapsed().as_micros() as f64);
    }

    let mean = timings.iter().sum::<f64>() / timings.len() as f64;
    let min = timings.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = timings.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let variance =
        timings.iter().map(|t| (t - mean).powi(2)).sum::<f64>() / timings.len() as f64;
    let stddev = variance.sqrt();

    let fmt_us = |us: f64| -> String {
        if us < 1000.0 {
            format!("{:.1}µs", us)
        } else if us < 1_000_000.0 {
            format!("{:.2}ms", us / 1000.0)
        } else {
            format!("{:.3}s", us / 1_000_000.0)
        }
    };

    println!("  Mean:    {}", fmt_us(mean));
    println!("  Min:     {}", fmt_us(min));
    println!("  Max:     {}", fmt_us(max));
    println!("  Std dev: {}", fmt_us(stddev));
    println!("  Runs:    {}", runs);

    let mut regression = false;
    if let Some((prev_mean, prev_min, prev_max, prev_stddev)) = prev {
        const REGRESSION_THRESHOLD: f64 = 10.0; // percent

        let delta_pct = |cur: f64, old: f64| -> f64 {
            if old == 0.0 { 0.0 } else { (cur - old) / old * 100.0 }
        };
        let fmt_delta = |cur: f64, old: f64| -> String {
            if old == 0.0 { return "n/a".to_string(); }
            let pct = delta_pct(cur, old);
            if pct > REGRESSION_THRESHOLD {
                format!("+{:.1}% slower ⚠ REGRESSION", pct)
            } else if pct > 0.0 {
                format!("+{:.1}% slower", pct)
            } else {
                format!("{:.1}% faster", -pct)
            }
        };

        let mean_pct = delta_pct(mean, prev_mean);
        if mean_pct > REGRESSION_THRESHOLD {
            regression = true;
        }

        println!("\n  ┌─────────────────────────────────────────────────┐");
        println!("  │ Regression Check (threshold: {}%)               │", REGRESSION_THRESHOLD as u32);
        println!("  ├──────────┬──────────────┬──────────────┬────────┤");
        println!("  │ Metric   │ Baseline     │ Current      │ Delta  │");
        println!("  ├──────────┼──────────────┼──────────────┼────────┤");
        println!("  │ Mean     │ {:>12} │ {:>12} │ {}",
            fmt_us(prev_mean), fmt_us(mean), fmt_delta(mean, prev_mean));
        println!("  │ Min      │ {:>12} │ {:>12} │ {}",
            fmt_us(prev_min), fmt_us(min), fmt_delta(min, prev_min));
        println!("  │ Max      │ {:>12} │ {:>12} │ {}",
            fmt_us(prev_max), fmt_us(max), fmt_delta(max, prev_max));
        println!("  │ Std dev  │ {:>12} │ {:>12} │",
            fmt_us(prev_stddev), fmt_us(stddev));
        println!("  └──────────┴──────────────┴──────────────┘");

        if regression {
            eprintln!("\n  ✗ REGRESSION DETECTED: mean regressed {:.1}% (threshold: {}%)",
                mean_pct, REGRESSION_THRESHOLD as u32);
        } else {
            println!("\n  ✓ No regression (mean within {}% threshold)", REGRESSION_THRESHOLD as u32);
        }
    }

    if let Some(save_path) = save {
        let json = format!(
            "{{\"mean_us\":{:.3},\"min_us\":{:.3},\"max_us\":{:.3},\"stddev_us\":{:.3},\"runs\":{},\"file\":\"{}\"}}",
            mean, min, max, stddev, runs, file.display()
        );
        fs::write(save_path, json)?;
        println!("\n  Results saved to {}", save_path.display());
    }

    if regression {
        std::process::exit(1);
    }
    Ok(())
}

/// Compute the regression delta % between current and baseline mean.
/// Positive = slower (regression), negative = faster.
pub fn regression_delta_pct(current_mean_us: f64, baseline_mean_us: f64) -> f64 {
    if baseline_mean_us == 0.0 {
        0.0
    } else {
        (current_mean_us - baseline_mean_us) / baseline_mean_us * 100.0
    }
}

/// Returns true if the regression exceeds the threshold (default 10%).
pub fn is_regression(current_mean_us: f64, baseline_mean_us: f64) -> bool {
    regression_delta_pct(current_mean_us, baseline_mean_us) > 10.0
}

/// Parse a minimal bench JSON: returns (mean, min, max, stddev) in microseconds.
fn parse_bench_json(data: &str) -> Option<(f64, f64, f64, f64)> {
    let get = |key: &str| -> Option<f64> {
        let needle = format!("\"{}\":", key);
        let pos = data.find(&needle)? + needle.len();
        let rest = data[pos..].trim_start();
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        rest[..end].trim().parse().ok()
    };
    Some((
        get("mean_us")?,
        get("min_us")?,
        get("max_us")?,
        get("stddev_us")?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bench_json_valid() {
        let json = r#"{"mean_us":850.0,"min_us":820.0,"max_us":950.0,"stddev_us":35.0,"runs":20}"#;
        let result = parse_bench_json(json);
        assert!(result.is_some());
        let (mean, min, max, stddev) = result.unwrap();
        assert!((mean - 850.0).abs() < 0.1);
        assert!((min - 820.0).abs() < 0.1);
        assert!((max - 950.0).abs() < 0.1);
        assert!((stddev - 35.0).abs() < 0.1);
    }

    #[test]
    fn test_parse_bench_json_invalid_returns_none() {
        assert!(parse_bench_json("{}").is_none());
        assert!(parse_bench_json("not json").is_none());
    }

    #[test]
    fn test_regression_delta_pct_slower() {
        // 10% slower
        let delta = regression_delta_pct(110.0, 100.0);
        assert!((delta - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_regression_delta_pct_faster() {
        // 5% faster
        let delta = regression_delta_pct(95.0, 100.0);
        assert!((delta - (-5.0)).abs() < 0.01);
    }

    #[test]
    fn test_regression_delta_zero_baseline() {
        assert_eq!(regression_delta_pct(100.0, 0.0), 0.0);
    }

    #[test]
    fn test_is_regression_above_threshold() {
        assert!(is_regression(115.0, 100.0), "15% slower should be a regression");
    }

    #[test]
    fn test_is_regression_below_threshold() {
        assert!(!is_regression(108.0, 100.0), "8% slower should not be a regression");
    }

    #[test]
    fn test_is_regression_at_boundary() {
        // Exactly 10% — NOT a regression (threshold is strictly >10%)
        assert!(!is_regression(110.0, 100.0), "Exactly 10% should not be a regression");
        // Just above
        assert!(is_regression(110.1, 100.0), "10.1% should be a regression");
    }

    #[test]
    fn test_baseline_json_parses() {
        // Verify the committed baseline.json is valid
        let baseline_path = std::path::PathBuf::from("benches/baseline.json");
        if baseline_path.exists() {
            let content = std::fs::read_to_string(&baseline_path).unwrap();
            assert!(content.contains("mean_us"), "baseline.json should contain mean_us entries");
            assert!(content.contains("fib"), "baseline.json should contain fib benchmark");
        }
    }
}

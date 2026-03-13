//! `txtcode bench` — micro-benchmark a Txt-code program.

use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::runtime::vm::VirtualMachine;
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

    println!("Benchmarking: {}", file.display());
    println!("  Warmup: {} run(s), measured: {} run(s)\n", warmup, runs);

    for _ in 0..warmup {
        let mut vm = VirtualMachine::new();
        vm.interpret(&program)
            .map_err(|e| format!("Runtime error: {}", e))?;
    }

    let mut timings: Vec<f64> = Vec::with_capacity(runs);
    for _ in 0..runs {
        let start = std::time::Instant::now();
        let mut vm = VirtualMachine::new();
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

    if let Some((prev_mean, prev_min, prev_max, prev_stddev)) = prev {
        println!("\n  Comparison vs baseline:");
        let delta = |cur: f64, old: f64| {
            if old == 0.0 {
                return "n/a".to_string();
            }
            let pct = (cur - old) / old * 100.0;
            if pct > 0.0 {
                format!("+{:.1}% slower", pct)
            } else {
                format!("{:.1}% faster", -pct)
            }
        };
        println!(
            "  Mean:    {} → {} ({})",
            fmt_us(prev_mean),
            fmt_us(mean),
            delta(mean, prev_mean)
        );
        println!(
            "  Min:     {} → {} ({})",
            fmt_us(prev_min),
            fmt_us(min),
            delta(min, prev_min)
        );
        println!(
            "  Max:     {} → {} ({})",
            fmt_us(prev_max),
            fmt_us(max),
            delta(max, prev_max)
        );
        println!("  Std dev: {} → {}", fmt_us(prev_stddev), fmt_us(stddev));
    }

    if let Some(save_path) = save {
        let json = format!(
            "{{\"mean_us\":{:.3},\"min_us\":{:.3},\"max_us\":{:.3},\"stddev_us\":{:.3},\"runs\":{},\"file\":\"{}\"}}",
            mean, min, max, stddev, runs, file.display()
        );
        fs::write(save_path, json)?;
        println!("\n  Results saved to {}", save_path.display());
    }

    Ok(())
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

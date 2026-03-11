# Txtcode Examples

All examples in this directory are verified to run with `txtcode run` (v0.4).

| Example | Run command | Demonstrates |
|---------|-------------|--------------|
| `hello_world.tc` | `txtcode run examples/hello_world.tc` | Variables, functions, loops, maps, error handling |
| `calculator.tc` | `txtcode run examples/calculator.tc` | `match` dispatch, store+return pattern |
| `file_processing.tc` | `txtcode run examples/file_processing.tc` | CSV parsing, stats, tokenizer |
| `log_analyzer.tc` | `txtcode run examples/log_analyzer.tc` | Log classification, counters, structured output |
| `pipeline.tc` | `txtcode run examples/pipeline.tc` | Sequential task runner, abort-on-failure |
| `audit_trail.tc` | `txtcode run --allow-fs /tmp/ examples/audit_trail.tc` | Path-guarded file I/O, permission model |
| `config_validator.tc` | `txtcode run examples/config_validator.tc` | Multi-rule validation, error aggregation |
| `directory_summary.tc` | `txtcode run --allow-fs . examples/directory_summary.tc` | Filesystem scan, file sizes, ranking |
| `metrics_report.tc` | `txtcode run examples/metrics_report.tc` | Map grouping, aggregation, stats |
| `math_utils.tc` | `txtcode run examples/math_utils.tc` | Numeric functions, recursion |
| `security_demo.tc` | `txtcode run examples/security_demo.tc` | SHA-256, AES-GCM encrypt/decrypt, base64 |

## Notes

- `audit_trail.tc` and `directory_summary.tc` require `--allow-fs` because they read/write the filesystem.
- Examples use only confirmed-stable stdlib functions for v0.4.
- See [`experimental/README.md`](experimental/README.md) for examples that use planned (not-yet-implemented) features.

# Txtcode Examples

All examples in this directory are verified to run with `txtcode run` (v0.5.0).

## Core Examples

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

## v0.5.0 Feature Examples

These examples demonstrate new stdlib functions added in v0.5.0:

```txtcode
// Streaming file I/O (file_open / file_read_line)
store → h → file_open("data.txt", "r")
store → line → file_read_line(h)
while → line != null
  print → line
  store → line → file_read_line(h)
end
file_close(h)

// Datetime helpers
store → today → format_datetime(now(), "%Y-%m-%d", "UTC")
store → tomorrow → datetime_add(now(), 1, "days")
store → diff → datetime_diff(tomorrow, now(), "hours")

// CSV write
csv_write("/tmp/report.csv", [["name","score"],["Alice",95],["Bob",87]])

// Process piping
store → result → exec_pipe(["echo hello world", "tr a-z A-Z"])

// HTTP server (requires net feature)
// http_serve(8080, (req) → http_response(200, "OK", {}))
```

## Notes

- Examples requiring filesystem access (`audit_trail.tc`, `directory_summary.tc`) need `--allow-fs`.
- Examples requiring network access need `--allow-net=<host>`.
- All stdlib functions used are stable as of v0.5.0.
- See [`experimental/README.md`](experimental/README.md) for examples using planned features.

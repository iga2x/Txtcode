# Experimental Examples

These examples target language features or stdlib functions that are **not yet supported
in Txtcode v0.4**. They are kept here for reference and to track planned work.

Running them with `txtcode run` will produce a parse error or runtime error.

## Why they're here

| File | Unsupported feature | Planned |
|------|---------------------|---------|
| `config_parser.tc` | Nested map assignment `m["a"]["b"] → val` | v0.5 |
| `module_import.tc` | `from Module import name` keyword syntax | v0.5 |
| `package_usage.tc` | Package-level import parse errors | v0.5 |
| `policy_exec.tc` | Multi-arg `permission → sys, "exec", "/usr/*"` syntax; `capability →` keyword | v0.5 |
| `port_scanner.tc` | `capability → network.tcp.connect` keyword syntax | v0.5 |
| `task_automation.tc` | Subscript-then-call `task["handler"]()` expression | v0.5 |
| `web_server.tc` | `capability →` keyword; `http_serve()` not in stdlib | v0.5 |

## Stable examples (in `examples/`)

These all pass with `txtcode run`:

- `hello_world.tc` — variables, functions, loops, maps, error handling
- `calculator.tc` — match dispatch, store+return pattern
- `file_processing.tc` — CSV parsing, stats, tokenizer
- `log_analyzer.tc` — log classification, counters, string operations
- `pipeline.tc` — sequential task runner with abort-on-failure
- `audit_trail.tc` — path-guarded file auditor (`txtcode run --allow-fs /tmp/`)
- `math_utils.tc` — numeric functions, recursion
- `security_demo.tc` — sha256, encrypt/decrypt, base64

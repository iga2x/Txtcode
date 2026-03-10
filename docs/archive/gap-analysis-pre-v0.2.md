Txt-code — Full Gap Analysis & What's Missing                                                                                                                                                                          
                                                                                                                                                                                                                         
  ---                                                                                                                                                                                                                    
  SECTION 1 — AI-Only Features to Generalize or Remove                                                                                                                                                                   
                                                                                                                                                                                                                         
  You said: avoid things that are only usable by AI. Here are the things currently built only for AI use that need to be generalized or removed:                                                                         
                                                                                                                                                                                                                       
  intent declaration — too AI-specific

  define → fetch → (url)
    intent → "Fetch a URL and return body"   ← this exists only for AI agents to read
  end
  Problem: Normal developers don't use this. It serves no runtime purpose.
  Fix needed: Either remove it, or rename it to purpose / doc that serves as general documentation metadata (like a docstring in Python). Then it has real value for everyone.

  ai_hint declaration — too AI-specific

  define → parse → (input)
    ai_hint → "Input must be valid JSON"   ← purely AI metadata
  end
  Problem: Only useful if an AI model reads your source. Useless for human developers.
  Fix needed: Rename to hint and make it appear in linter output and docgen output. Then humans benefit too.

  AIMetadata struct inside the VM — internal AI tracking

  File: src/runtime/audit.rs and src/runtime/vm.rs
  Problem: The VM carries ai_metadata: AIMetadata which is a concept baked into the core runtime. This makes the runtime coupled to AI workflows.
  Fix needed: Replace with generic metadata: HashMap<String, String> that any caller can populate. AI agents can use it, but it doesn't assume AI is always present.

  CapabilityExecutor trait — partially AI-oriented

  Some of the capability system (tool_exec, tool_list, tool_info) is designed around AI orchestration patterns.
  Fix needed: Keep the tools system, but document it as a general plugin/extension system, not an AI feature.

  ---
  SECTION 2 — Virtual Environment (Like Python's venv)

  Where Python stands vs where Txt-code stands right now:

  ┌────────────────────────────────┬─────────────────────────────────┬──────────────────────────────────────────────┐
  │            Feature             │           Python venv           │              Txt-code (current)              │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Project-local packages         │ ✅ .venv/lib/                   │ ❌ Global only ~/.txtcode/packages/          │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Activate/deactivate shell      │ ✅ source .venv/bin/activate    │ ❌ Not implemented                           │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Isolated Python/binary version │ ✅ per-env                      │ ❌ Not implemented                           │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Project lockfile               │ ✅ requirements.txt/poetry.lock │ ✅ Txtcode.lock exists                       │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Manifest file                  │ ✅ pyproject.toml               │ ✅ Txtcode.toml exists                       │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Module path override           │ ✅ automatic                    │ ⚠️  TXTCODE_MODULE_PATH env var (manual only) │
  ├────────────────────────────────┼─────────────────────────────────┼──────────────────────────────────────────────┤
  │ Env isolation for security     │ ✅                              │ ❌ No isolation at all                       │
  └────────────────────────────────┴─────────────────────────────────┴──────────────────────────────────────────────┘

  What currently exists toward a venv system:

  ~/.txtcode/
  ├── packages/     ← global package install (like site-packages without venv)
  ├── cache/        ← compiled .txtc bytecode cache
  ├── logs/         ← audit trail logs
  └── config.toml   ← global config

  Txtcode.toml and Txtcode.lock already exist per-project — the foundation is there. But packages still install globally, not per-project.

  What is needed to complete a venv system:

  Stage 1 — Project-local packages (minimal venv)
  txtcode env create              # creates .txtcode-env/ in current directory
  txtcode env install             # installs Txtcode.toml deps into .txtcode-env/
  txtcode run script.tc           # auto-detects .txtcode-env/, uses local packages

  Stage 2 — Activation like Python
  source $(txtcode env path)      # sets TXTCODE_MODULE_PATH to .txtcode-env/
  txtcode run script.tc           # uses local env
  txtcode env deactivate          # clears env vars

  Stage 3 — Safe execution sandbox
  Run scripts in a fully isolated environment where they cannot touch the real filesystem unless you say so — like a container but lightweight:
  txtcode run --sandbox script.tc         # no real filesystem access
  txtcode run --sandbox --allow-fs=/tmp   # only /tmp accessible
  txtcode run --sandbox --allow-net       # network allowed

  Current status: The infrastructure (config, lockfile, module path env var) is ~30% there. No actual isolation exists yet.

  ---
  SECTION 3 — Missing Language Features

  3.1 Not Implemented at All

  ┌─────────────────────────────────────┬─────────────────────────────────────────┬──────────┐
  │               Feature               │               What it is                │ Priority │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Result<T, E> type                   │ Built-in success/failure type like Rust │ High     │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Stack depth limit                   │ Prevent infinite recursion crashes      │ High     │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Integer overflow guards             │ i64 overflow is undefined now           │ High     │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ for loop in bytecode VM             │ Iterator support in compiled code       │ High     │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ break/continue in bytecode VM       │ Loop control in compiled code           │ High     │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Function scoping in bytecode VM     │ Functions don't store separately        │ High     │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Closures in bytecode VM             │ Captured variables not working          │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Async/await in bytecode VM          │ All async ignored in compiled mode      │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ String interpolation in bytecode VM │ f"Hello {name}" broken in bytecode      │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Slice expressions in bytecode VM    │ arr[1:4] not compiled                   │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Module imports in bytecode VM       │ import → math broken in compiled mode   │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ ++/-- everywhere                    │ Increment/decrement                     │ Low      │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Struct literal syntax at runtime    │ Point { x: 1, y: 2 }                    │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Enum variant matching               │ Full pattern matching on enums          │ Medium   │
  ├─────────────────────────────────────┼─────────────────────────────────────────┼──────────┤
  │ Generic type enforcement            │ define → id<T>(x: T) types not checked  │ Low      │
  └─────────────────────────────────────┴─────────────────────────────────────────┴──────────┘

  3.2 Partially Implemented

  ┌──────────────────────┬───────────────────────┬───────────────────────────────────────────────────┐
  │       Feature        │      What works       │                   What doesn't                    │
  ├──────────────────────┼───────────────────────┼───────────────────────────────────────────────────┤
  │ ?? null coalescing   │ Bytecode VM now works │ AST VM needs verification                         │
  ├──────────────────────┼───────────────────────┼───────────────────────────────────────────────────┤
  │ Optional chaining ?. │ AST VM works          │ Bytecode VM raises error                          │
  ├──────────────────────┼───────────────────────┼───────────────────────────────────────────────────┤
  │ Pattern matching     │ Basic cases           │ Nested patterns, struct destructuring             │
  ├──────────────────────┼───────────────────────┼───────────────────────────────────────────────────┤
  │ do-while loop        │ AST VM                │ Bytecode VM has a bug (condition direction wrong) │
  ├──────────────────────┼───────────────────────┼───────────────────────────────────────────────────┤
  │ Module system        │ Import/export works   │ Version headers not actually parsed               │
  ├──────────────────────┼───────────────────────┼───────────────────────────────────────────────────┤
  │ Async/await          │ Tokio-backed          │ No error propagation from await                   │
  └──────────────────────┴───────────────────────┴───────────────────────────────────────────────────┘

  3.3 Language Features Completely Absent (Not Designed Yet)

  ┌──────────────────────────────┬───────────────────────────────────────────┐
  │           Feature            │                Description                │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ type aliases                 │ type UserId = int                         │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Named error types            │ error → NotFound → ("Resource not found") │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Interface/protocol           │ Define what methods a type must have      │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Pipe operator                │ data |> transform |> output               │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Destructured function args   │ define → f → ({x, y})                     │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Multi-return values          │ return → a, b                             │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Spread operator              │ [...arr1, ...arr2]                        │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Default export               │ Module-level export → default             │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Conditional import           │ import if platform == "linux"             │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ String raw literals          │ r"no\nescape"                             │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Multi-line strings           │ """...""" block strings                   │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Hex/binary literals in float │ 0x1.8p+1 style                            │
  ├──────────────────────────────┼───────────────────────────────────────────┤
  │ Number separators            │ 1_000_000 for readability                 │
  └──────────────────────────────┴───────────────────────────────────────────┘

  ---
  SECTION 4 — Missing stdlib Functions

  Network

  ┌───────────────────────────────────┬──────────────────────────────┐
  │              Missing              │         Description          │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ http_put, http_delete, http_patch │ REST methods beyond GET/POST │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ http_headers                      │ Read response headers        │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ http_status                       │ Read HTTP status code        │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ websocket_connect                 │ WebSocket client             │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ http_stream                       │ Streaming HTTP response      │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ http_timeout                      │ Set request timeout          │
  ├───────────────────────────────────┼──────────────────────────────┤
  │ Cookie/session management         │ Auth flows                   │
  └───────────────────────────────────┴──────────────────────────────┘

  File I/O

  ┌─────────────────────────┬────────────────────────────────────┐
  │         Missing         │            Description             │
  ├─────────────────────────┼────────────────────────────────────┤
  │ watch_file              │ File change detection              │
  ├─────────────────────────┼────────────────────────────────────┤
  │ read_lines              │ Read file line by line (streaming) │
  ├─────────────────────────┼────────────────────────────────────┤
  │ read_csv                │ Parse CSV files                    │
  ├─────────────────────────┼────────────────────────────────────┤
  │ temp_file               │ Create a temporary file safely     │
  ├─────────────────────────┼────────────────────────────────────┤
  │ file_size               │ Get file size                      │
  ├─────────────────────────┼────────────────────────────────────┤
  │ file_modified           │ Get last modified time             │
  ├─────────────────────────┼────────────────────────────────────┤
  │ zip_create, zip_extract │ Archive handling                   │
  ├─────────────────────────┼────────────────────────────────────┤
  │ symlink_create          │ Create symbolic links              │
  └─────────────────────────┴────────────────────────────────────┘

  System

  ┌─────────────────────┬────────────────────────────────┐
  │       Missing       │          Description           │
  ├─────────────────────┼────────────────────────────────┤
  │ env_list            │ List all environment variables │
  ├─────────────────────┼────────────────────────────────┤
  │ signal_send         │ Send signal to a process       │
  ├─────────────────────┼────────────────────────────────┤
  │ pipe_exec           │ Execute and pipe stdin/stdout  │
  ├─────────────────────┼────────────────────────────────┤
  │ which               │ Find binary in PATH            │
  ├─────────────────────┼────────────────────────────────┤
  │ is_root             │ Check if running as root       │
  ├─────────────────────┼────────────────────────────────┤
  │ cpu_count           │ Number of CPU cores            │
  ├─────────────────────┼────────────────────────────────┤
  │ memory_available    │ Available RAM                  │
  ├─────────────────────┼────────────────────────────────┤
  │ disk_space          │ Disk usage                     │
  ├─────────────────────┼────────────────────────────────┤
  │ os_name, os_version │ Detailed OS info               │
  └─────────────────────┴────────────────────────────────┘

  String/Data

  ┌──────────────────────────────┬────────────────────────────────┐
  │           Missing            │          Description           │
  ├──────────────────────────────┼────────────────────────────────┤
  │ str_pad_left, str_pad_right  │ Padding                        │
  ├──────────────────────────────┼────────────────────────────────┤
  │ str_wrap                     │ Word wrap at width             │
  ├──────────────────────────────┼────────────────────────────────┤
  │ str_dedent                   │ Remove common indentation      │
  ├──────────────────────────────┼────────────────────────────────┤
  │ str_count                    │ Count occurrences of substring │
  ├──────────────────────────────┼────────────────────────────────┤
  │ base32_encode, base32_decode │ Base32 encoding                │
  ├──────────────────────────────┼────────────────────────────────┤
  │ yaml_encode, yaml_decode     │ YAML support                   │
  ├──────────────────────────────┼────────────────────────────────┤
  │ toml_encode, toml_decode     │ TOML support                   │
  ├──────────────────────────────┼────────────────────────────────┤
  │ csv_encode, csv_decode       │ CSV support                    │
  ├──────────────────────────────┼────────────────────────────────┤
  │ xml_parse                    │ Basic XML/HTML parsing         │
  ├──────────────────────────────┼────────────────────────────────┤
  │ html_escape                  │ HTML entity encoding           │
  └──────────────────────────────┴────────────────────────────────┘

  Math

  ┌────────────────────┬─────────────────────────────┐
  │      Missing       │         Description         │
  ├────────────────────┼─────────────────────────────┤
  │ math_clamp         │ Clamp value between min/max │
  ├────────────────────┼─────────────────────────────┤
  │ math_lerp          │ Linear interpolation        │
  ├────────────────────┼─────────────────────────────┤
  │ math_gcd, math_lcm │ GCD and LCM                 │
  ├────────────────────┼─────────────────────────────┤
  │ math_factorial     │ Factorial                   │
  ├────────────────────┼─────────────────────────────┤
  │ math_combinations  │ Combinatorics               │
  ├────────────────────┼─────────────────────────────┤
  │ math_random_int    │ Random integer in range     │
  ├────────────────────┼─────────────────────────────┤
  │ math_random_float  │ Random float 0.0–1.0        │
  └────────────────────┴─────────────────────────────┘

  Crypto (Security)

  ┌────────────────────────────────────┬─────────────────────────────────┐
  │              Missing               │           Description           │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ hmac_sha256                        │ HMAC signing                    │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ bcrypt_hash, bcrypt_verify         │ Password hashing                │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ rsa_generate, rsa_sign, rsa_verify │ RSA keypair                     │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ ed25519_sign, ed25519_verify       │ Ed25519 signing                 │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ uuid_v4                            │ UUID generation                 │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ secure_compare                     │ Constant-time string comparison │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ pbkdf2                             │ Key derivation                  │
  └────────────────────────────────────┴─────────────────────────────────┘

  ---
  SECTION 5 — Missing CLI Commands and Options

  Commands that are documented but not smoke-tested or partially wired:

  ┌──────────────────────┬──────────────────────────────────────────────────────────┐
  │       Command        │                          Issue                           │
  ├──────────────────────┼──────────────────────────────────────────────────────────┤
  │ txtcode init [name]  │ Unclear if it creates files or just config               │
  ├──────────────────────┼──────────────────────────────────────────────────────────┤
  │ txtcode debug <file> │ Wired to debugger but no tests                           │
  ├──────────────────────┼──────────────────────────────────────────────────────────┤
  │ txtcode bench <file> │ No output format, no comparison mode                     │
  ├──────────────────────┼──────────────────────────────────────────────────────────┤
  │ txtcode test         │ What does it actually run? No test runner format defined │
  ├──────────────────────┼──────────────────────────────────────────────────────────┤
  │ txtcode doc          │ HTML output wraps in <pre> which is wrong                │
  └──────────────────────┴──────────────────────────────────────────────────────────┘

  Missing CLI options:

  ┌──────────────────────────────────┬───────────────────────────────────────────────┐
  │              Option              │                  Description                  │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode run --timeout 30s        │ Max execution time                            │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode run --memory 512mb       │ Memory limit                                  │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode run --sandbox            │ Filesystem/network isolation                  │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode run --env-file .env      │ Load environment from file                    │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode run --no-color           │ Disable colored output                        │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode run --json               │ Output errors as JSON (for CI/AI consumption) │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode compile --optimize 2     │ Expose optimization level in compile          │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode lint --format json       │ Machine-readable lint output                  │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode lint --fix               │ Auto-fix simple lint issues                   │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode format --check           │ Exit non-zero if formatting needed (CI mode)  │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode test --filter pattern    │ Run only matching tests                       │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode test --watch             │ Re-run on file change                         │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode bench --compare old.txtc │ Compare benchmark results                     │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode package search <name>    │ Search available packages                     │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode package info <name>      │ Show package details                          │
  ├──────────────────────────────────┼───────────────────────────────────────────────┤
  │ txtcode package remove <name>    │ Remove a dependency                           │
  └──────────────────────────────────┴───────────────────────────────────────────────┘

  ---
  SECTION 6 — Permission System Gaps

  What needs to be added to make the permission system complete:

  ┌───────────────────────────────────────────┬────────────────────────────────────────────────────────────────┬─────────────────────────────────────────┐
  │                    Gap                    │                            Problem                             │               Fix needed                │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ Bytecode VM has NO permissions            │ Compiled scripts bypass all security                           │ Wire PermissionChecker into bytecode VM │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ No default-deny mode                      │ Currently nothing is granted, but some things slip through     │ Explicit --allow-nothing starting point │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ No network allowlist                      │ Can block all net, but can't say "allow only example.com"      │ Host-level filtering in NetLib          │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ No filesystem allowlist                   │ Can block all fs, but can't say "only /tmp/" cleanly           │ Path filter in IOLib                    │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ No permission inheritance                 │ Nested function calls don't inherit parent permissions cleanly │ Scoped permission stack                 │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ No rate limit on file ops                 │ Rate limits exist for net but not file I/O                     │ Extend policy engine                    │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ Audit log has no integrity check          │ Anyone can edit the log file                                   │ HMAC signature on each log entry        │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ No permission prompt                      │ Script just errors; could ask user to allow                    │ Interactive permission prompt option    │
  ├───────────────────────────────────────────┼────────────────────────────────────────────────────────────────┼─────────────────────────────────────────┤
  │ Capability tokens not enforced end-to-end │ use_capability / grant_capability wiring incomplete            │ Complete capability token lifecycle     │
  └───────────────────────────────────────────┴────────────────────────────────────────────────────────────────┴─────────────────────────────────────────┘

  ---
  SECTION 7 — Runtime Gaps

  ┌─────────────────────────────────────┬───────────────────────────────────┬───────────────────────────────────────────────┐
  │                 Gap                 │              Impact               │                     Notes                     │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ No garbage collector                │ Memory only grows; no collection  │ GC stub exists, not wired                     │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ No tail call optimization           │ Deep recursion always crashes     │ Common in functional patterns                 │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ No coroutines                       │ Async is only at function level   │ No yield/generator support                    │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ No FFI (Foreign Function Interface) │ Can't call C/Rust libraries       │ Major limitation for systems use              │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ No multi-threading                  │ Everything runs on one thread     │ Even async is single-threaded                 │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ Float equality bug                  │ 0.1 + 0.2 == 0.3 fails silently   │ Spec says epsilon, but implementation may not │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ No numeric literal separators       │ 1000000 hard to read vs 1_000_000 │ Minor but real                                │
  ├─────────────────────────────────────┼───────────────────────────────────┼───────────────────────────────────────────────┤
  │ Division produces int for int/float │ 5 / 2 == 2 (truncates), not 2.5   │ Could surprise users from Python              │
  └─────────────────────────────────────┴───────────────────────────────────┴───────────────────────────────────────────────┘

  ---
  SECTION 8 — Tooling Gaps

  Formatter

  ┌────────────────────────────────────┬─────────────────────────────────┐
  │              Missing               │           Description           │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ Multi-file formatting with summary │ How many files changed          │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ --check mode for CI                │ Exit 1 if files need formatting │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ Configurable max line length       │ Currently hardcoded             │
  ├────────────────────────────────────┼─────────────────────────────────┤
  │ Sort imports                       │ Alphabetize import statements   │
  └────────────────────────────────────┴─────────────────────────────────┘

  Linter

  ┌───────────────────────────┬────────────────────────────────────────────┐
  │          Missing          │                Description                 │
  ├───────────────────────────┼────────────────────────────────────────────┤
  │ --fix auto-fix mode       │ Fix simple issues automatically            │
  ├───────────────────────────┼────────────────────────────────────────────┤
  │ JSON/SARIF output         │ For CI integration                         │
  ├───────────────────────────┼────────────────────────────────────────────┤
  │ Custom rule configuration │ Enable/disable specific checks             │
  ├───────────────────────────┼────────────────────────────────────────────┤
  │ Security-specific checks  │ Detect exec without permission declaration │
  ├───────────────────────────┼────────────────────────────────────────────┤
  │ Dead code detection       │ Functions defined but never called         │
  ├───────────────────────────┼────────────────────────────────────────────┤
  │ Complexity checks         │ Functions that are too long or complex     │
  └───────────────────────────┴────────────────────────────────────────────┘

  Debugger

  ┌─────────────────────────┬─────────────────────────────────────┐
  │         Missing         │             Description             │
  ├─────────────────────────┼─────────────────────────────────────┤
  │ Watch expressions       │ Auto-print variable when it changes │
  ├─────────────────────────┼─────────────────────────────────────┤
  │ Conditional breakpoints │ Break only when x > 10              │
  ├─────────────────────────┼─────────────────────────────────────┤
  │ Reverse execution       │ Step backward through history       │
  ├─────────────────────────┼─────────────────────────────────────┤
  │ Memory view             │ Show all live values                │
  ├─────────────────────────┼─────────────────────────────────────┤
  │ Network request log     │ Show all HTTP calls made            │
  └─────────────────────────┴─────────────────────────────────────┘

  Package Manager

  ┌────────────────────────┬──────────────────────────────────────┐
  │        Missing         │             Description              │
  ├────────────────────────┼──────────────────────────────────────┤
  │ Package search         │ Find packages by name/keyword        │
  ├────────────────────────┼──────────────────────────────────────┤
  │ Package publish        │ Upload your own package              │
  ├────────────────────────┼──────────────────────────────────────┤
  │ Private registry       │ Internal company packages            │
  ├────────────────────────┼──────────────────────────────────────┤
  │ Package signing        │ Verify package authenticity          │
  ├────────────────────────┼──────────────────────────────────────┤
  │ Vulnerability check    │ Warn about known vulnerable packages │
  ├────────────────────────┼──────────────────────────────────────┤
  │ Project-local install  │ Install to ./packages/ not global    │
  ├────────────────────────┼──────────────────────────────────────┤
  │ txtcode package remove │ Remove a package                     │
  └────────────────────────┴──────────────────────────────────────┘

  ---
  SECTION 9 — Missing Documentation

  ┌──────────────────────────┬──────────────────────────────────────────────────────┐
  │         Missing          │                     Description                      │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Interactive playground   │ Run Txt-code in the browser                          │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Tutorial series          │ Getting started step by step                         │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Cookbook                 │ Common patterns (HTTP server, file processing, etc.) │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Error code reference     │ Every RuntimeError with fix suggestions              │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ stdlib API reference     │ Every function with arguments, return type, examples │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Security hardening guide │ How to run scripts safely in production              │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Contribution guide       │ How to add a new stdlib function                     │
  ├──────────────────────────┼──────────────────────────────────────────────────────┤
  │ Module authoring guide   │ How to write and publish a package                   │
  └──────────────────────────┴──────────────────────────────────────────────────────┘

  ---
  SECTION 10 — Priority Stack Ranking

  Here is everything ranked by how important it is to do before the language is usable in production:

  Must-do (language not production-safe without these)

  1. Stack depth limit — OS crash on recursion is unacceptable
  2. Integer overflow guards — undefined behavior is a security risk
  3. Bytecode VM permission enforcement — current compiled code is unsandboxed
  4. for loop + break/continue in bytecode VM — basic control flow broken
  5. Function scoping in bytecode VM — fundamental feature broken in compiled mode
  6. Project-local packages (venv stage 1) — global package install causes conflicts

  Should-do (important for real-world use)

  7. Result<T,E> type — error handling is clunky without it
  8. --sandbox mode — true execution isolation
  9. HTTP PUT/DELETE/PATCH — half the REST verbs are missing
  10. txtcode format --check for CI — formatters need CI mode
  11. txtcode lint --format json — machine-readable output
  12. Audit log integrity (HMAC) — audit trail can be tampered
  13. str_pad, str_wrap, YAML, TOML stdlib — common everyday needs

  Nice-to-have (quality of life)

  14. Pipe operator |> — cleaner data pipelines
  15. Number separators 1_000_000
  16. Multi-line strings """
  17. Debugger watch expressions
  18. Package search / publish
  19. Interactive docs playground
  20. Generalize intent/ai_hint → purpose/hint

  ---
  Summary Picture

  CORE RUNTIME          ████████░░  80%  — missing stack limit, overflow, GC
  BYTECODE VM           ████░░░░░░  40%  — missing loops, functions, closures, modules
  PERMISSION SYSTEM     ██████░░░░  60%  — works in AST VM, broken in bytecode VM
  STDLIB                ██████░░░░  60%  — core good, many functions missing
  CLI TOOLING           ███████░░░  70%  — commands exist, options incomplete
  VIRTUAL ENV           ██░░░░░░░░  20%  — lockfile exists, no isolation
  PACKAGE MANAGER       █████░░░░░  50%  — install works, no search/publish/local
  DOCUMENTATION         ███████░░░  70%  — spec good, no playground/cookbook
  TESTING               ████░░░░░░  40%  — unit tests exist, no E2E, no fuzzing
  SECURITY HARDENING    █████░░░░░  50%  — AST VM good, bytecode VM open

  ---
  That is the complete picture of everything missing, incomplete, or needing generalization — with no development started yet, just the full inventory so you know exactly where to direct work next.

✻ Sautéed for 2m 3s

─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
❯  
─────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────────
  ⏵⏵ accept edits on (shift+tab to cycle)                                                                                                                                                                ⧉ In README.md
                                                                                                                                         ✗ Auto-update failed · Try claude doctor or npm i -g @anthropic-ai/claude-code

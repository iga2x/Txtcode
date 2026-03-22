# Txtcode Plugin SDK

Txtcode plugins are shared libraries (`.so` on Linux, `.dylib` on macOS, `.dll` on Windows)
that expose a JSON-based calling convention so they can be loaded and called from Txtcode
scripts via `plugin_load` / `plugin_call`.

## Plugin ABI

Every plugin must export the following C symbols:

```c
// Return a null-terminated plugin name string (static lifetime — do not free).
const char *txtcode_plugin_name(void);

// Optional: return a null-terminated array of null-terminated function names.
// The array and all strings must have static lifetime.
const char **txtcode_functions(void);

// Each exported function follows this signature:
//   args_json — null-terminated UTF-8 JSON array of arguments
//   returns   — heap-allocated null-terminated UTF-8 JSON value
//               (Txtcode calls txtcode_free_result to release memory)
char *<function_name>(const char *args_json);

// Release memory returned by any function above.
void txtcode_free_result(char *result);
```

## C Example

```c
#include <stdlib.h>
#include <string.h>
#include <stdio.h>

const char *txtcode_plugin_name(void) {
    return "my_plugin";
}

static const char *FUNCTION_NAMES[] = { "add", NULL };

const char **txtcode_functions(void) {
    return FUNCTION_NAMES;
}

// add(a, b) → a + b
char *add(const char *args_json) {
    // Minimal JSON parse: expect [number, number]
    double a = 0, b = 0;
    sscanf(args_json, "[%lf,%lf]", &a, &b);
    char *result = malloc(64);
    snprintf(result, 64, "%.17g", a + b);
    return result;
}

void txtcode_free_result(char *result) {
    free(result);
}
```

Compile as a shared library:

```bash
cc -shared -fPIC -o my_plugin.so my_plugin.c
```

## Rust Example

```rust
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

#[no_mangle]
pub extern "C" fn txtcode_plugin_name() -> *const c_char {
    b"my_rust_plugin\0".as_ptr() as *const c_char
}

static FUNCTION_NAMES: &[*const c_char] = &[
    b"greet\0".as_ptr() as *const c_char,
    std::ptr::null(),
];

#[no_mangle]
pub extern "C" fn txtcode_functions() -> *const *const c_char {
    FUNCTION_NAMES.as_ptr()
}

/// greet(name) → "Hello, <name>!"
#[no_mangle]
pub extern "C" fn greet(args_json: *const c_char) -> *mut c_char {
    let args = unsafe { CStr::from_ptr(args_json) }.to_string_lossy();
    // args is a JSON array, e.g. ["Alice"]
    let name = args.trim_matches(['[', ']', '"', ' ']);
    let result = format!("\"Hello, {}!\"", name);
    CString::new(result).unwrap().into_raw()
}

#[no_mangle]
pub extern "C" fn txtcode_free_result(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)); }
    }
}
```

Add to `Cargo.toml`:
```toml
[lib]
crate-type = ["cdylib"]
```

## Usage from Txtcode

```
// Requires --allow-ffi=/path/to/my_plugin.so
store → handle → plugin_load("/path/to/my_plugin.so")
store → result → plugin_call(handle, "add", [3, 4])
// result = 7
```

## Permissions

`plugin_load` and `plugin_call` require the `sys.ffi` permission and the library path
must be in the FFI allowlist passed via `--allow-ffi`:

```bash
txtcode run --allow-ffi=/path/to/my_plugin.so script.tc
```

## Feature Gate

The plugin system requires the `ffi` feature:

```bash
cargo build --features ffi
```

Without this feature, `plugin_load` returns a clear error:
`"plugin_load requires the 'ffi' feature. Rebuild with: cargo build --features ffi"`

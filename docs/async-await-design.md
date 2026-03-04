# Async/Await Design for Txtcode

## Syntax Design

### Async Functions
```txtcode
# Define async function
async -> define -> fetch_data (url: string) -> string
    # async function body
    return -> result
end

# Or with arrow syntax
async define -> fetch_data (url: string) -> string
    return -> result
end
```

### Await Expressions
```txtcode
# Await a function call
store -> result -> await -> http_get(url)

# Await in expression
store -> data -> await -> fetch_data("https://api.example.com/data")

# Await in assignment
store -> response -> await -> http_post(url, body)
```

### Async Function Calls
```txtcode
# Call async function (returns a Future)
store -> future -> fetch_data("https://example.com")

# Await the future
store -> result -> await -> future

# Or directly
store -> result -> await -> fetch_data("https://example.com")
```

## Grammar Updates

```
function_def     → ("async" "→")? "define" "→" identifier "→" "(" parameters? ")" ("→" type)? statement* "end"
await_expression → "await" "→" expression
```

## Type System

- Async functions return `Future<T>` where T is the return type
- `await` unwraps a `Future<T>` to `T`
- Type checker ensures `await` is only used on `Future` types

## Runtime Implementation

- Use Tokio for async runtime
- Async functions are compiled to return `Future<Value>`
- VM has async executor that can run futures
- Standard library async functions use Tokio's async I/O

## Examples

```txtcode
# Async HTTP request
async define -> fetch_json (url: string) -> map
    store -> response -> await -> http_get(url)
    return -> json_decode(response)
end

# Parallel requests
async define -> fetch_multiple (urls: array[string]) -> array[map]
    store -> futures -> map(urls, (url) -> http_get(url))
    store -> results -> await -> all(futures)
    return -> results
end

# Async file I/O
async define -> read_file_async (path: string) -> string
    return -> await -> read_file(path)
end
```


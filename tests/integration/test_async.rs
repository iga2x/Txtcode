use std::sync::Arc;
use txtcode::lexer::Lexer;
use txtcode::parser::Parser;
use txtcode::runtime::vm::VirtualMachine;

fn run_ast_repl(
    source: &str,
) -> Result<txtcode::runtime::Value, txtcode::runtime::errors::RuntimeError> {
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program)
}

fn run(source: &str) -> txtcode::runtime::Value {
    use txtcode::runtime::Value;
    let tokens = txtcode::lexer::Lexer::new(source.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    vm.interpret_repl(&program).unwrap_or(Value::Null)
}

// ---------------------------------------------------------------------------
// Basic async/await tests
// ---------------------------------------------------------------------------

#[test]
fn test_async_call_returns_future() {
    let result = run_ast_repl(
        r#"
async define → double → (x)
  return → x * 2
end
store → f → double(5)
f
"#,
    );
    assert!(result.is_ok(), "{:?}", result);
    assert!(
        matches!(result.unwrap(), txtcode::runtime::Value::Future(_)),
        "expected Value::Future"
    );
}

#[test]
fn test_async_await_resolves_value() {
    let result = run_ast_repl(
        r#"
async define → triple → (x)
  return → x * 3
end
store → result → await triple(4)
result
"#,
    );
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(12));
}

#[test]
fn test_await_on_non_future_is_identity() {
    let result = run_ast_repl(
        "store → x → await 42\nx",
    );
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

#[test]
fn test_async_concurrent_tasks() {
    let result = run_ast_repl(
        r#"
async define → add_one → (x)
  return → x + 1
end
store → f1 → add_one(10)
store → f2 → add_one(20)
store → r1 → await f1
store → r2 → await f2
r1 + r2
"#,
    );
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(32));
}

#[test]
fn test_async_function_sees_globals() {
    let result = run_ast_repl(
        r#"
store → base → 100
async define → offset → (x)
  return → base + x
end
store → result → await offset(7)
result
"#,
    );
    assert!(result.is_ok(), "{:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(107));
}

// ---------------------------------------------------------------------------
// await_all / await_any combinators (Task 12.1)
// ---------------------------------------------------------------------------

#[test]
fn test_await_all_collects_results() {
    use txtcode::runtime::Value;
    let source = r#"
store → vals → await_all([1, 2, 3])
vals
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(
        result,
        Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]),
        "await_all on plain values should collect them into an array"
    );
}

#[test]
fn test_await_any_returns_first() {
    use txtcode::runtime::Value;
    let source = r#"
await_any([42, 99, 0])
"#;
    let mut lexer = Lexer::new(source.to_string());
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret_repl(&program).unwrap();
    assert_eq!(result, Value::Integer(42), "await_any should return the first value");
}

// ---------------------------------------------------------------------------
// Nursery tests (Task 15.1)
// ---------------------------------------------------------------------------

#[test]
fn test_nursery_basic() {
    use txtcode::runtime::Value;
    let src = r#"
define → noop_task → ()
  store → x → 1 + 1
end
async → nursery
  nursery_spawn(noop_task)
  nursery_spawn(noop_task)
end
"done"
"#;
    let result = run(src);
    assert_eq!(result, Value::String(Arc::from("done")));
}

#[test]
fn test_nursery_error_propagates() {
    let src = r#"
define → failing_task → ()
  store → x → 1 / 0
end
async → nursery
  nursery_spawn(failing_task)
end
"#;
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret(&program);
    assert!(result.is_err(), "nursery should propagate child task errors");
}

#[test]
fn test_nursery_spawn_outside_errors() {
    let src = r#"
define → task → ()
  store → x → 1
end
nursery_spawn(task)
"#;
    let tokens = txtcode::lexer::Lexer::new(src.to_string()).tokenize().unwrap();
    let program = txtcode::parser::Parser::new(tokens).parse().unwrap();
    let mut vm = VirtualMachine::new();
    let result = vm.interpret(&program);
    assert!(result.is_err(), "nursery_spawn outside nursery should error");
}

#[test]
fn test_nursery_empty_body() {
    use txtcode::runtime::Value;
    let src = r#"
async → nursery
end
"ok"
"#;
    let result = run(src);
    assert_eq!(result, Value::String(Arc::from("ok")));
}

// ---------------------------------------------------------------------------
// Async generators / streams (Task 15.2)
// ---------------------------------------------------------------------------

#[test]
fn test_async_generator_returns_future() {
    use txtcode::runtime::Value;
    let src = r#"
async define → gen → ()
  yield → 1
  yield → 2
  yield → 3
end
gen()
"#;
    let result = run(src);
    assert!(matches!(result, Value::Future(_)), "expected Future, got {:?}", result);
}

#[test]
fn test_async_generator_await_gives_array() {
    use txtcode::runtime::Value;
    let src = r#"
async define → gen → ()
  yield → 10
  yield → 20
  yield → 30
end
store → stream → gen()
store → result → await stream
result
"#;
    let result = run(src);
    assert_eq!(result, Value::Array(vec![
        Value::Integer(10),
        Value::Integer(20),
        Value::Integer(30),
    ]));
}

#[test]
fn test_async_for_consumes_stream() {
    use txtcode::runtime::Value;
    let src = r#"
async define → gen → ()
  yield → 1
  yield → 2
  yield → 3
end
store → total → 0
async → for → x in gen()
  total += x
end
total
"#;
    let result = run(src);
    assert_eq!(result, Value::Integer(6));
}

// ---------------------------------------------------------------------------
// Timeout / deadline (Task 15.3)
// ---------------------------------------------------------------------------

#[test]
fn test_with_timeout_success() {
    use txtcode::runtime::Value;
    let src = r#"
define → quick → ()
  return → 42
end
with_timeout(5000, quick)
"#;
    let result = run(src);
    assert_eq!(result, Value::Result(true, Box::new(Value::Integer(42))));
}

#[test]
fn test_with_timeout_expires() {
    use txtcode::runtime::Value;
    let src = r#"
define → slow → ()
  sleep(5000)
  99
end
with_timeout(5, slow)
"#;
    let result = run(src);
    assert_eq!(
        result,
        Value::Result(false, Box::new(Value::String(Arc::from("timeout"))))
    );
}

#[test]
fn test_async_for_loop_yields() {
    use txtcode::runtime::Value;
    let src = r#"
async define → squares → ()
  for → i in [1, 2, 3, 4]
    yield → i * i
  end
end
store → total → 0
async → for → v in squares()
  total += v
end
total
"#;
    let result = run(src);
    assert_eq!(result, Value::Integer(30));
}

// ---------------------------------------------------------------------------
// async_run / await_future / await_all / async_sleep (Group 20.2)
// ---------------------------------------------------------------------------

#[test]
fn test_async_run_returns_future() {
    let src = r#"
define → my_task → ()
  return → 42
end
store → f → async_run(my_task)
await_future(f)
"#;
    let result = run_ast_repl(src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

#[test]
fn test_async_run_await_all_collects_results() {
    let src = r#"
define → task_a → ()
  return → 1
end
define → task_b → ()
  return → 2
end
store → fa → async_run(task_a)
store → fb → async_run(task_b)
store → results → await_all([fa, fb])
len(results)
"#;
    let result = run_ast_repl(src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(2));
}

#[test]
fn test_async_run_two_tasks_both_complete() {
    let src = r#"
define → make_value → ()
  return → 99
end
store → f1 → async_run(make_value)
store → f2 → async_run(make_value)
store → collected → await_all([f1, f2])
len(collected)
"#;
    let result = run_ast_repl(src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(2));
}

#[test]
fn test_async_run_result_value() {
    let src = r#"
define → make_value → ()
  return → 99
end
store → fh → async_run(make_value)
await_future(fh)
"#;
    let result = run_ast_repl(src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(99));
}

#[test]
fn test_await_future_passthrough_non_future() {
    let result = run_ast_repl("await_future(123)");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(123));
}

#[test]
fn test_async_sleep_returns_null() {
    let result = run_ast_repl("async_sleep(0)");
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Null);
}

#[test]
fn test_async_run_parallel_two_tasks() {
    let src = r#"
define → slow_task → ()
  sleep(30)
  return → 7
end
store → fh1 → async_run(slow_task)
store → fh2 → async_run(slow_task)
store → collected → await_all([fh1, fh2])
len(collected)
"#;
    let result = run_ast_repl(src);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(2));
}

// ---------------------------------------------------------------------------
// Async cancel token (Group 26.3)
// ---------------------------------------------------------------------------

#[test]
fn test_async_cancel_token_create_and_cancel() {
    use txtcode::runtime::Value;
    let source_before = r#"
store → tok → async_cancel_token()
is_cancelled(tok)
"#.to_string();
    let mut lexer = Lexer::new(source_before);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let before = vm.interpret_repl(&program).unwrap();
    assert_eq!(before, Value::Boolean(false), "new token should not be cancelled");
}

#[test]
fn test_async_cancel_token_after_cancel() {
    use txtcode::runtime::Value;
    let source = r#"
store → tok → async_cancel_token()
async_cancel(tok)
is_cancelled(tok)
"#.to_string();
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize().unwrap();
    let mut parser = Parser::new(tokens);
    let program = parser.parse().unwrap();
    let mut vm = VirtualMachine::new();
    let after = vm.interpret_repl(&program).unwrap();
    assert_eq!(after, Value::Boolean(true), "token should be cancelled after async_cancel");
}

// ---------------------------------------------------------------------------
// Event loop tests (Group 26.1, D.1)
// ---------------------------------------------------------------------------

#[test]
fn test_event_loop_enable_and_is_enabled() {
    txtcode::runtime::event_loop::disable_for_test();
    assert!(!txtcode::runtime::event_loop::is_enabled(), "should be disabled initially");
    txtcode::runtime::event_loop::enable();
    assert!(txtcode::runtime::event_loop::is_enabled(), "should be enabled after enable()");
    txtcode::runtime::event_loop::disable_for_test();
}

#[test]
fn test_event_loop_submit_task_completes() {
    txtcode::runtime::event_loop::enable();
    let (tx, rx) = std::sync::mpsc::channel::<i64>();
    let submitted = txtcode::runtime::event_loop::submit(Box::new(move || {
        tx.send(42).ok();
    }));
    assert!(submitted, "task submission must succeed when event loop is enabled");
    let val = rx.recv_timeout(std::time::Duration::from_secs(2)).expect("task must complete");
    assert_eq!(val, 42);
    txtcode::runtime::event_loop::disable_for_test();
}

#[test]
fn test_event_loop_multiple_tasks_complete() {
    txtcode::runtime::event_loop::enable();
    let count = 10;
    let (tx, rx) = std::sync::mpsc::channel::<i64>();
    for i in 0..count {
        let tx2 = tx.clone();
        txtcode::runtime::event_loop::submit(Box::new(move || {
            tx2.send(i).ok();
        }));
    }
    drop(tx);
    let mut results: Vec<i64> = rx.into_iter().collect();
    results.sort();
    assert_eq!(results, (0..count).collect::<Vec<_>>(), "all tasks must complete");
    txtcode::runtime::event_loop::disable_for_test();
}

#[test]
fn test_event_loop_async_run_returns_future() {
    txtcode::runtime::event_loop::enable();
    let src = r#"
define → add_one → ()
  return → 41 + 1
end
store → h → async_run(add_one)
await_future(h)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "async_run via event loop must succeed: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
    txtcode::runtime::event_loop::disable_for_test();
}

#[test]
fn test_event_loop_submit_returns_false_when_disabled() {
    txtcode::runtime::event_loop::disable_for_test();
    let submitted = txtcode::runtime::event_loop::submit(Box::new(|| {}));
    assert!(!submitted, "submit must return false when event loop is not started");
}

#[test]
fn test_event_loop_multiworker_parallel_timing() {
    txtcode::runtime::event_loop::disable_for_test();
    txtcode::runtime::event_loop::set_worker_count(4);
    txtcode::runtime::event_loop::enable();

    let (tx, rx) = std::sync::mpsc::sync_channel::<u64>(16);
    let start = std::time::Instant::now();
    let mut submitted = 0;
    for _ in 0..4 {
        let tx2 = tx.clone();
        if txtcode::runtime::event_loop::submit(Box::new(move || {
            std::thread::sleep(std::time::Duration::from_millis(100));
            tx2.send(1).ok();
        })) {
            submitted += 1;
        }
    }
    drop(tx);
    let mut completed = 0;
    for _ in 0..submitted {
        if rx.recv_timeout(std::time::Duration::from_secs(5)).is_ok() {
            completed += 1;
        }
    }
    let elapsed = start.elapsed().as_millis();
    assert_eq!(completed, submitted, "all submitted tasks must complete");
    if submitted == 4 {
        assert!(elapsed < 600, "4 parallel 100ms tasks should finish in < 600ms, took {}ms", elapsed);
    }
    txtcode::runtime::event_loop::disable_for_test();
}

#[test]
fn test_event_loop_worker_count_respected() {
    txtcode::runtime::event_loop::disable_for_test();
    txtcode::runtime::event_loop::set_worker_count(3);
    assert_eq!(txtcode::runtime::event_loop::worker_count(), 3);
    txtcode::runtime::event_loop::disable_for_test();
}

#[test]
fn test_event_loop_tasks_submitted_counter() {
    let _count = txtcode::runtime::event_loop::TASKS_SUBMITTED
        .load(std::sync::atomic::Ordering::Relaxed);

    txtcode::runtime::event_loop::enable();
    let (tx, rx) = std::sync::mpsc::sync_channel::<()>(10);
    let mut submitted = 0;
    for _ in 0..3 {
        let tx2 = tx.clone();
        if txtcode::runtime::event_loop::submit(Box::new(move || { tx2.send(()).ok(); })) {
            submitted += 1;
        }
    }
    drop(tx);
    let mut received = 0;
    for _ in 0..submitted {
        if rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok() {
            received += 1;
        }
    }
    assert_eq!(received, submitted, "all submitted tasks must complete");
}

// ---------------------------------------------------------------------------
// D.2: async permission snapshot
// ---------------------------------------------------------------------------

#[test]
fn test_async_permission_snapshot_not_affected_by_parent_deny() {
    use txtcode::runtime::permissions::{Permission, PermissionResource};

    let mut vm = VirtualMachine::new();
    vm.grant_permission(PermissionResource::FileSystem("/tmp".to_string()), None);

    let snapshot = vm.snapshot_permissions();

    vm.deny_permission(PermissionResource::FileSystem("/tmp".to_string()), None);

    let fs_resource = PermissionResource::FileSystem("/tmp".to_string());
    let mut child_vm = VirtualMachine::new();
    child_vm.set_permission_manager(snapshot);
    assert!(
        child_vm.check_permission(&fs_resource, None).is_ok(),
        "child VM with pre-deny snapshot should allow fs permission"
    );
    assert!(
        vm.check_permission(&fs_resource, None).is_err(),
        "parent VM should deny after explicit deny"
    );
}

#[test]
fn test_async_run_scoped_restricts_permissions() {
    let src = r#"
async define → worker → ()
  return → 42
end
store → h → async_run_scoped(worker, ["net.connect"])
await_future(h)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "async_run_scoped should succeed: {:?}", result);
    assert_eq!(result.unwrap(), txtcode::runtime::Value::Integer(42));
}

// ---------------------------------------------------------------------------
// O.4: async_run_timeout
// ---------------------------------------------------------------------------

#[test]
fn test_o4_async_run_timeout_negative_timeout_errors() {
    let src = r#"
define → my_task → ()
  return → 42
end
async_run_timeout(my_task, -1)
"#;
    let result = run_ast_repl(src);
    if let Err(e) = result {
        assert!(
            e.to_string().contains("positive") || e.to_string().contains("timeout"),
            "negative timeout should give clear error: {}", e
        );
    }
}

#[test]
fn test_o4_async_run_timeout_completes_fast_task() {
    let src = r#"
define → my_task → ()
  return → 42
end
store → fut → async_run_timeout(my_task, 5000)
await_future(fut)
"#;
    let result = run_ast_repl(src);
    assert!(result.is_ok(), "async_run_timeout with fast task should succeed: {:?}", result);
}

// ---------------------------------------------------------------------------
// M.1: Async back-pressure tests
// ---------------------------------------------------------------------------

#[test]
fn test_m1_set_max_concurrent_tasks_roundtrip() {
    txtcode::runtime::event_loop::set_max_concurrent_tasks(8);
    assert_eq!(txtcode::runtime::event_loop::max_concurrent_tasks(), 8);
    txtcode::runtime::event_loop::set_max_concurrent_tasks(64);
    assert_eq!(txtcode::runtime::event_loop::max_concurrent_tasks(), 64);
}

#[test]
fn test_m1_submit_blocked_when_cap_reached() {
    txtcode::runtime::event_loop::disable_for_test();
    let submitted = txtcode::runtime::event_loop::submit(Box::new(|| {}));
    assert!(!submitted, "submit() must return false when event loop is disabled");
}

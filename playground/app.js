/**
 * Task 28.3 — Txtcode Web REPL Playground
 *
 * Loads the Txtcode WASM module and wires it to the CodeMirror editor.
 * URL sharing: ?code=<base64> pre-fills the editor.
 */

'use strict';

// ── CodeMirror setup ──────────────────────────────────────────────────────────

const editorTextarea = document.getElementById('code-input');
const output = document.getElementById('output');
const runBtn = document.getElementById('run-btn');
const shareBtn = document.getElementById('share-btn');
const status = document.getElementById('status');

let editor;
if (typeof CodeMirror !== 'undefined') {
  editor = CodeMirror.fromTextArea(editorTextarea, {
    lineNumbers: true,
    theme: 'default',
    mode: 'null',        // no specific mode until txtcode grammar ships
    indentWithTabs: false,
    tabSize: 2,
    lineWrapping: false,
    autofocus: true,
  });
  // Stretch to fill pane
  editor.getWrapperElement().style.flex = '1';
  editor.getWrapperElement().style.overflow = 'hidden';
  editor.refresh();
}

function getCode() {
  return editor ? editor.getValue() : editorTextarea.value;
}

// ── URL sharing (base64 ?code=) ───────────────────────────────────────────────

function loadFromUrl() {
  const params = new URLSearchParams(window.location.search);
  const encoded = params.get('code');
  if (encoded) {
    try {
      const code = atob(encoded);
      if (editor) editor.setValue(code);
      else editorTextarea.value = code;
    } catch (e) { /* ignore invalid base64 */ }
  } else {
    const defaultCode = [
      '// Welcome to the Txtcode Playground!',
      '// Try editing the code below and clicking Run.',
      '',
      'store → name → "world"',
      'println(f"Hello, {name}!")',
      '',
      'store → result → 0',
      'for → i in range(1, 6)',
      '  store → result → result + i',
      'end',
      '',
      'println(f"Sum 1..5 = {result}")',
    ].join('\n');
    if (editor) editor.setValue(defaultCode);
    else editorTextarea.value = defaultCode;
  }
}

shareBtn.addEventListener('click', () => {
  const encoded = btoa(getCode());
  const url = `${window.location.origin}${window.location.pathname}?code=${encoded}`;
  navigator.clipboard.writeText(url).then(() => {
    shareBtn.textContent = 'Copied!';
    setTimeout(() => { shareBtn.textContent = 'Share'; }, 2000);
  });
});

// ── WASM loading ──────────────────────────────────────────────────────────────

let evalScript = null;

async function loadWasm() {
  try {
    // wasm-bindgen generated glue
    const wasmGlue = './txtcode_playground.js';
    const mod = await import(wasmGlue).catch(() => null);
    if (mod) {
      await mod.default('./txtcode_playground_bg.wasm');
      evalScript = mod.eval_script;
      status.textContent = 'Ready';
      status.style.color = '#4ade80';
    } else {
      // Fallback: try bare WebAssembly API
      const response = await fetch('./txtcode_playground.wasm');
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      const wasmBytes = await response.arrayBuffer();
      const { instance } = await WebAssembly.instantiate(wasmBytes, {
        env: {
          // Minimal host imports required by the Txtcode runtime
          print_i64: (n) => appendOutput(String(n)),
          print_f64: (f) => appendOutput(String(f)),
          print_str: () => {},
          array_new: () => 0n,
          array_get: () => 0n,
          array_len: () => 0n,
        }
      });
      const exp = instance.exports;
      if (typeof exp.eval_script === 'function') {
        evalScript = (src) => {
          // Pass string via linear memory (simplified — assumes ASCII for demo)
          const enc = new TextEncoder();
          const bytes = enc.encode(src + '\0');
          const ptr = exp.__wbindgen_malloc ? exp.__wbindgen_malloc(bytes.length) : 0;
          if (ptr) {
            new Uint8Array(exp.memory.buffer).set(bytes, ptr);
            const result = exp.eval_script(ptr, bytes.length - 1);
            return result ? new TextDecoder().decode(
              new Uint8Array(exp.memory.buffer, result).slice(0, 256)) : '{"error":"no result"}';
          }
          return '{"error":"memory not available"}';
        };
        status.textContent = 'Ready (bare WASM)';
        status.style.color = '#fbbf24';
      } else {
        throw new Error('eval_script not exported from WASM');
      }
    }
  } catch (err) {
    console.warn('WASM load failed, using mock evaluator:', err);
    status.textContent = 'Demo mode (no WASM)';
    status.style.color = '#f87171';
    // Provide a JS-based mock so the UI is functional without the WASM build
    evalScript = mockEval;
  }
  runBtn.disabled = false;
  loadFromUrl();
}

// ── Mock evaluator (fallback when WASM not available) ─────────────────────────

function mockEval(src) {
  // Very limited subset for demonstration
  const lines = src.split('\n').map(l => l.trim()).filter(l => l && !l.startsWith('//'));
  let out = [];
  const vars = {};
  for (const line of lines) {
    const printMatch = line.match(/^println\s*\(\s*f?"([^"]*)"\s*\)/);
    if (printMatch) {
      const text = printMatch[1].replace(/\{(\w+)\}/g, (_, v) => vars[v] ?? `{${v}}`);
      out.push(text);
      continue;
    }
    const storeMatch = line.match(/^store\s*→\s*(\w+)\s*→\s*(.+)/);
    if (storeMatch) {
      const [, name, expr] = storeMatch;
      const numMatch = expr.match(/^-?\d+(\.\d+)?$/);
      vars[name] = numMatch ? Number(expr) : expr.replace(/^"|"$/g, '');
    }
  }
  if (out.length) return JSON.stringify({ ok: out.join('\n') });
  return JSON.stringify({ ok: '(no output)' });
}

// ── Output helpers ────────────────────────────────────────────────────────────

function appendOutput(text, cls = '') {
  const line = document.createElement('span');
  if (cls) line.className = cls;
  line.textContent = text;
  output.appendChild(line);
  output.appendChild(document.createTextNode('\n'));
  output.scrollTop = output.scrollHeight;
}

function clearOutput() {
  output.innerHTML = '';
}

// ── Run handler ───────────────────────────────────────────────────────────────

runBtn.addEventListener('click', () => {
  clearOutput();
  const code = getCode();
  if (!code.trim()) {
    appendOutput('(empty input)', 'info');
    return;
  }
  try {
    const raw = evalScript(code);
    const result = typeof raw === 'string' ? JSON.parse(raw) : raw;
    if (result.ok !== undefined) {
      appendOutput(result.ok, 'ok');
    } else if (result.error !== undefined) {
      appendOutput('Error: ' + result.error, 'err');
    } else {
      appendOutput(JSON.stringify(result), 'info');
    }
  } catch (e) {
    appendOutput('Playground error: ' + e.message, 'err');
  }
});

// Ctrl+Enter / Cmd+Enter shortcut
window.addEventListener('keydown', (e) => {
  if ((e.ctrlKey || e.metaKey) && e.key === 'Enter' && !runBtn.disabled) {
    runBtn.click();
  }
});

// ── Init ──────────────────────────────────────────────────────────────────────

loadWasm();

let wasmExports = null;

export async function initWasm() {
  if (wasmExports) return wasmExports;

  const [wasmBinary, jsModule] = await Promise.all([
    fetch('/wasm/pipeql_wasm_bg.wasm').then(r => r.arrayBuffer()),
    import(/* @vite-ignore */ '/wasm/pipeql_wasm.js'),
  ]);

  const wasmModule = await WebAssembly.compile(wasmBinary);
  jsModule.initSync(wasmModule);
  wasmExports = jsModule;
  return wasmExports;
}

export async function compile(source, dialect) {
  if (!wasmExports) {
    await initWasm();
  }
  if (!wasmExports || typeof wasmExports.compile !== 'function') {
    throw new Error('WASM compiler module not ready');
  }
  const compiled = wasmExports.compile(source, dialect || 'postgres');
  const result = {
    sql: compiled.sql,
    params: Array.from(compiled.params),
    statementType: compiled.statement_type,
    isMutation: compiled.is_mutation,
    parameterCount: compiled.parameter_count,
    analysis: compiled.analysis,
  };
  compiled.free();
  return result;
}

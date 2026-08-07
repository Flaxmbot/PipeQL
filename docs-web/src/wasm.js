let wasmExports = null;

export async function initWasm() {
  if (wasmExports) return;

  const [wasmBinary, jsModule] = await Promise.all([
    fetch('/wasm/pipeql_wasm_bg.wasm').then(r => r.arrayBuffer()),
    import(/* @vite-ignore */ '/wasm/pipeql_wasm.js'),
  ]);

  const wasmModule = await WebAssembly.compile(wasmBinary);
  jsModule.initSync(wasmModule);
  wasmExports = jsModule;
}

export function compile(source, dialect) {
  if (!wasmExports) throw new Error('WASM not initialized');
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

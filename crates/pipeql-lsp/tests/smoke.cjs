// LSP protocol smoke test for pipeql-lsp.
// Sends JSON-RPC messages over stdio and checks the responses.
const { spawn } = require("child_process");

const lsp = "D:/PipeQL/target/release/pipeql-lsp.exe";

function frame(obj) {
  const body = JSON.stringify(obj);
  return `Content-Length: ${Buffer.byteLength(body)}\r\n\r\n${body}`;
}

function parseFrames(buf) {
  const frames = [];
  let rest = buf;
  for (;;) {
    const headerEnd = rest.indexOf("\r\n\r\n");
    if (headerEnd === -1) break;
    const header = rest.slice(0, headerEnd).toString();
    const m = /Content-Length: (\d+)/.exec(header);
    if (!m) break;
    const len = parseInt(m[1], 10);
    if (rest.length < headerEnd + 4 + len) break;
    const body = rest.slice(headerEnd + 4, headerEnd + 4 + len).toString();
    try {
      frames.push(JSON.parse(body));
    } catch (e) {
      frames.push({ parseError: String(e), raw: body });
    }
    rest = rest.slice(headerEnd + 4 + len);
  }
  return { frames, rest };
}

let buffer = Buffer.alloc(0);
const received = [];
const pending = new Map();

const child = spawn(lsp, [], { stdio: ["pipe", "pipe", "pipe"] });
child.stdout.on("data", (chunk) => {
  buffer = Buffer.concat([buffer, chunk]);
  const { frames, rest } = parseFrames(buffer);
  buffer = rest;
  for (const f of frames) {
    received.push(f);
    if (f.id !== undefined && pending.has(f.id)) {
      pending.get(f.id)(f);
      pending.delete(f.id);
    }
  }
});
child.stderr.on("data", (d) => {
  console.error("LSP stderr:", d.toString().trim());
});

let nextId = 1;
function request(method, params) {
  const id = nextId++;
  return new Promise((resolve, reject) => {
    pending.set(id, resolve);
    child.stdin.write(frame({ jsonrpc: "2.0", id, method, params }));
    setTimeout(() => {
      if (pending.has(id)) {
        pending.delete(id);
        reject(new Error(`timeout waiting for ${method}`));
      }
    }, 5000);
  });
}
function notify(method, params) {
  child.stdin.write(frame({ jsonrpc: "2.0", method, params }));
}

(async () => {
  const failures = [];
  const check = (name, cond, detail) => {
    console.log(`${cond ? "ok" : "FAIL"}: ${name}${cond ? "" : " -> " + detail}`);
    if (!cond) failures.push(name);
  };

  const init = await request("initialize", {
    processId: null,
    rootUri: null,
    capabilities: {},
  });
  check(
    "initialize returns capabilities",
    !!init.result && !!init.result.capabilities,
    JSON.stringify(init),
  );
  check(
    "server reports full text sync",
    init.result.capabilities.textDocumentSync === 1,
    JSON.stringify(init.result.capabilities.textDocumentSync),
  );
  check(
    "server reports completion provider",
    !!init.result.capabilities.completionProvider,
    "missing",
  );
  check(
    "server reports hover provider",
    init.result.capabilities.hoverProvider === true,
    "missing",
  );

  notify("initialized", {});

  // Open a document with a parse error -> expect diagnostics.
  notify("textDocument/didOpen", {
    textDocument: {
      uri: "file:///tmp/bad.pql",
      languageId: "pipeql",
      version: 1,
      text: "from users | explode | take 3",
    },
  });
  await new Promise((r) => setTimeout(r, 500));

  const diag = received.filter((f) => f.method === "textDocument/publishDiagnostics");
  const bad = diag.find((f) => f.params.uri.endsWith("bad.pql"));
  check(
    "publishes parse diagnostics for bad query",
    !!bad && bad.params.diagnostics.length > 0,
    JSON.stringify(bad && bad.params),
  );
  check(
    "diagnostic mentions 'explode'",
    !!bad && bad.params.diagnostics[0].message.includes("explode"),
    JSON.stringify(bad && bad.params.diagnostics),
  );
  check(
    "diagnostic includes hint",
    !!bad && /hint:/.test(bad.params.diagnostics[0].message),
    "no hint",
  );

  // Open a good document -> expect no diagnostics.
  notify("textDocument/didOpen", {
    textDocument: {
      uri: "file:///tmp/good.pql",
      languageId: "pipeql",
      version: 2,
      text: "from users | filter age >= $min | select [id] | take 10",
    },
  });
  await new Promise((r) => setTimeout(r, 500));
  const good = received
    .filter((f) => f.method === "textDocument/publishDiagnostics")
    .find((f) => f.params.uri.endsWith("good.pql"));
  check(
    "no diagnostics for valid query",
    !!good && good.params.diagnostics.length === 0,
    JSON.stringify(good && good.params.diagnostics),
  );

  // Completion.
  const comp = await request("textDocument/completion", {
    textDocument: { uri: "file:///tmp/good.pql" },
    position: { line: 0, character: 0 },
  });
  const items = comp.result
    ? comp.result.items || comp.result
    : [];
  check("completion returns items", Array.isArray(items) && items.length > 0, JSON.stringify(comp));
  check(
    "completion includes 'filter'",
    Array.isArray(items) && items.some((i) => i.label === "filter"),
    "missing filter",
  );
  check(
    "completion includes 'select'",
    Array.isArray(items) && items.some((i) => i.label === "select"),
    "missing select",
  );
  check(
    "completion includes 'insert'",
    Array.isArray(items) && items.some((i) => i.label === "insert"),
    "missing insert",
  );
  check(
    "completion includes 'update'",
    Array.isArray(items) && items.some((i) => i.label === "update"),
    "missing update",
  );
  check(
    "completion includes 'delete'",
    Array.isArray(items) && items.some((i) => i.label === "delete"),
    "missing delete",
  );
  check(
    "completion includes 'table'",
    Array.isArray(items) && items.some((i) => i.label === "table"),
    "missing table",
  );
  check(
    "completion includes 'into'",
    Array.isArray(items) && items.some((i) => i.label === "into"),
    "missing into",
  );
  check(
    "completion includes 'string' type",
    Array.isArray(items) && items.some((i) => i.label === "string"),
    "missing string",
  );

  // Valid mutation document -> no diagnostics.
  notify("textDocument/didOpen", {
    textDocument: {
      uri: "file:///tmp/mut.pql",
      languageId: "pipeql",
      version: 3,
      text: "into notes | insert [title = $t, is_pinned = 1]",
    },
  });
  await new Promise((r) => setTimeout(r, 500));
  const mut = received
    .filter((f) => f.method === "textDocument/publishDiagnostics")
    .find((f) => f.params.uri.endsWith("mut.pql"));
  check(
    "no diagnostics for valid insert",
    !!mut && mut.params.diagnostics.length === 0,
    JSON.stringify(mut && mut.params.diagnostics),
  );

  // Invalid mutation (non-filter step before delete) -> analysis diagnostics.
  notify("textDocument/didOpen", {
    textDocument: {
      uri: "file:///tmp/badmut.pql",
      languageId: "pipeql",
      version: 4,
      text: "from notes | take 5 | delete",
    },
  });
  await new Promise((r) => setTimeout(r, 500));
  const badmut = received
    .filter((f) => f.method === "textDocument/publishDiagnostics")
    .find((f) => f.params.uri.endsWith("badmut.pql"));
  check(
    "publishes diagnostics for invalid mutation",
    !!badmut && badmut.params.diagnostics.length > 0,
    JSON.stringify(badmut && badmut.params.diagnostics),
  );

  // Hover.
  const hover = await request("textDocument/hover", {
    textDocument: { uri: "file:///tmp/good.pql" },
    position: { line: 0, character: 1 },
  });
  check("hover returns markup", !!hover.result && !!hover.result.contents, JSON.stringify(hover));

  await request("shutdown", null);
  notify("exit", null);

  setTimeout(() => {
    if (failures.length === 0) {
      console.log("\nALL LSP SMOKE TESTS PASSED");
      process.exit(0);
    } else {
      console.log(`\n${failures.length} LSP smoke test(s) FAILED`);
      process.exit(1);
    }
  }, 300);
})().catch((e) => {
  console.error("SMOKE ERROR:", e);
  process.exit(1);
});

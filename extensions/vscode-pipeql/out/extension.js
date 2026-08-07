"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.activate = activate;
exports.deactivate = deactivate;
const vscode = __importStar(require("vscode"));
const node_1 = require("vscode-languageclient/node");
let client;
function activate(context) {
    const config = vscode.workspace.getConfiguration("pipeql");
    if (config.get("lsp.enabled", true)) {
        const serverModule = resolveServerPath(context, config);
        if (serverModule) {
            const serverOptions = {
                run: { module: serverModule, transport: node_1.TransportKind.stdio },
                debug: { module: serverModule, transport: node_1.TransportKind.stdio },
            };
            const clientOptions = {
                documentSelector: [{ language: "pipeql" }],
                synchronize: {
                    configurationSection: "pipeql",
                },
            };
            client = new node_1.LanguageClient("pipeql", "PipeQL Language Server", serverOptions, clientOptions);
            client.start();
        }
        else {
            void vscode.window.showWarningMessage("PipeQL LSP binary not found. Install it with `cargo build -p pipeql-lsp` or set pipeql.lsp.path. Syntax highlighting still works.");
        }
    }
    context.subscriptions.push(vscode.commands.registerCommand("pipeql.compileToSql", () => {
        void compileActiveDocument();
    }));
}
function deactivate() {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
function resolveServerPath(context, config) {
    const configured = config.get("lsp.path", "");
    if (configured) {
        return configured;
    }
    const bundled = vscode.Uri.joinPath(context.extensionUri, "bin", process.platform === "win32" ? "pipeql-lsp.exe" : "pipeql-lsp");
    if (bundled && bundled.scheme === "file") {
        const fs = require("fs");
        if (fs.existsSync(bundled.fsPath)) {
            return bundled.fsPath;
        }
    }
    const candidates = ["pipeql-lsp"];
    if (process.platform === "win32") {
        candidates.push("pipeql-lsp.exe");
    }
    const childProcess = require("child_process");
    for (const name of candidates) {
        try {
            const result = childProcess.spawnSync(process.platform === "win32" ? "where" : "which", [name], { encoding: "utf8" });
            if (result.status === 0 && result.stdout.trim()) {
                return result.stdout.trim().split(/\r?\n/)[0];
            }
        }
        catch {
            // keep searching
        }
    }
    return undefined;
}
async function compileActiveDocument() {
    const editor = vscode.window.activeTextEditor;
    if (!editor || editor.document.languageId !== "pipeql") {
        return;
    }
    const config = vscode.workspace.getConfiguration("pipeql");
    const dialect = config.get("defaultDialect", "postgres");
    const childProcess = require("child_process");
    const cli = findCli();
    if (!cli) {
        void vscode.window.showErrorMessage("PipeQL CLI not found. Install it with `cargo install --path crates/pipeql-cli`.");
        return;
    }
    const result = childProcess.spawnSync(cli, 
    // `--` ends option parsing: query text often starts with a `--` comment.
    ["compile", "--dialect", dialect, "--json", "--", editor.document.getText()], { encoding: "utf8", maxBuffer: 16 * 1024 * 1024 });
    if (result.status !== 0) {
        void vscode.window.showErrorMessage(`PipeQL compile failed: ${(result.stderr || result.stdout).trim()}`);
        return;
    }
    const doc = await vscode.workspace.openTextDocument({
        language: "sql",
        content: result.stdout,
    });
    void vscode.window.showTextDocument(doc, vscode.ViewColumn.Beside, true);
}
function findCli() {
    const config = vscode.workspace.getConfiguration("pipeql");
    const configured = config.get("cliPath", "");
    if (configured) {
        return configured;
    }
    const candidates = ["pipeql", "pipeql.exe"];
    const childProcess = require("child_process");
    for (const name of candidates) {
        try {
            const result = childProcess.spawnSync(process.platform === "win32" ? "where" : "which", [name], { encoding: "utf8" });
            if (result.status === 0 && result.stdout.trim()) {
                return result.stdout.trim().split(/\r?\n/)[0];
            }
        }
        catch {
            // keep searching
        }
    }
    return undefined;
}
//# sourceMappingURL=extension.js.map
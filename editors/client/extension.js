'use strict';

const vscode = require('vscode');
const { LanguageClient, TransportKind } = require('vscode-languageclient/node');

let client;

/**
 * Activate the Txtcode VS Code extension.
 * Starts `txtcode lsp` as a language server and connects via stdin/stdout.
 */
function activate(context) {
    const config = vscode.workspace.getConfiguration('txtcode');
    const serverPath = config.get('serverPath', 'txtcode');

    const serverOptions = {
        command: serverPath,
        args: ['lsp'],
        transport: TransportKind.stdio,
    };

    const clientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'txtcode' },
            { scheme: 'untitled', language: 'txtcode' },
        ],
        synchronize: {
            fileEvents: vscode.workspace.createFileSystemWatcher('**/*.{tc,txtcode}'),
        },
        traceOutputChannel: vscode.window.createOutputChannel('Txtcode LSP Trace'),
    };

    client = new LanguageClient(
        'txtcode',
        'Txtcode Language Server',
        serverOptions,
        clientOptions
    );

    // Register commands
    context.subscriptions.push(
        vscode.commands.registerCommand('txtcode.restartServer', async () => {
            await client.stop();
            client.start();
            vscode.window.showInformationMessage('Txtcode: Language server restarted.');
        })
    );

    context.subscriptions.push(
        vscode.commands.registerCommand('txtcode.runFile', () => {
            const editor = vscode.window.activeTextEditor;
            if (!editor) {
                vscode.window.showWarningMessage('No active file to run.');
                return;
            }
            const filePath = editor.document.uri.fsPath;
            const terminal = vscode.window.createTerminal('Txtcode Run');
            terminal.show();
            terminal.sendText(`${serverPath} run "${filePath}"`);
        })
    );

    client.start();
    console.log('Txtcode language server started.');
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };

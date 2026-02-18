import { type ChildProcess, spawn } from "child_process";
import { ExtensionContext, TextDocumentContentProvider, Uri, ViewColumn } from "vscode";
import { commands, EventEmitter, window, workspace } from "vscode";
import type {
    LanguageClientOptions,
    ServerOptions,
} from "vscode-languageclient/node";

import { LanguageClient } from "vscode-languageclient/node";

let client: LanguageClient;
let server: ChildProcess | undefined;

const liveScheme = 'adept-live';

class LiveProvider implements TextDocumentContentProvider {
    private _onDidChange = new EventEmitter<Uri>();
    onDidChange = this._onDidChange.event;

    private content = new Map<string, string>();

    provideTextDocumentContent(uri: Uri): string {
        return this.content.get(uri.toString()) ?? '';
    }

    update(uri: Uri, text: string) {
        this.content.set(uri.toString(), text);
        this._onDidChange.fire(uri);
    }
}

async function startServer(context: ExtensionContext) {
    const config = workspace.getConfiguration("adept");
    const serverCommand: string = config.get("serverCommand") ?? "";

    if (serverCommand) {
        const languageId = "plaintext"; // "adept";
        const serverCommandArguments: string[] = config.get("serverCommandArguments") ?? [];
        const initializationOptions: object = config.get("initializationOptions") ?? {};

        const outputChannel = window.createOutputChannel("adept");
        outputChannel.appendLine("starting adept...");
        outputChannel.appendLine(
            JSON.stringify({ serverCommand, serverCommandArguments }),
        );

        const serverOptions: ServerOptions = (): Promise<ChildProcess> => {
            server = spawn(serverCommand, serverCommandArguments, {
                env: process.env,
                cwd: workspace.workspaceFolders?.[0]?.uri.fsPath,
            });

            server.on("error", (error) => {
                outputChannel.appendLine(
                    `Failed to start server: ${error.message}`,
                );
                window.showErrorMessage(
                    `Failed to start language server: ${serverCommand}. Error: ${error.message}`,
                );
            });

            server.on("exit", (code, signal) => {
                outputChannel.appendLine(
                    `Server process exited with code ${code} and signal ${signal}`,
                );
            });

            server.on("spawn", () => {
                window.showInformationMessage(
                    `Started language server: ${serverCommand}`,
                );
            });

            return Promise.resolve(server);
        };

        const clientOptions: LanguageClientOptions = {
            documentSelector: [languageId],
            diagnosticCollectionName: "adept",
            initializationOptions,
        };

        client = new LanguageClient(
            "adept",
            "Adept",
            serverOptions,
            clientOptions,
        );

        await client.start();
        outputChannel.appendLine("started adept.");

        const provider = new LiveProvider();

        context.subscriptions.push(
            workspace.registerTextDocumentContentProvider(liveScheme, provider)
        );

        context.subscriptions.push(
            commands.registerCommand('adept.showSyntaxTree', async () => {
                const fileUri = window.activeTextEditor?.document.uri.toString();
                if (fileUri == null) {
                    return;
                }

                const result = String(await client.sendRequest('workspace/executeCommand', {
                    command: 'adept.showSyntaxTree',
                    arguments: [fileUri]
                }));
                
                const uri = Uri.parse(liveScheme + ':Live Output');
                const doc = await workspace.openTextDocument(uri);

                await window.showTextDocument(doc, {
                    viewColumn: ViewColumn.Beside,
                    preview: false,
                    preserveFocus: true,
                });

                provider.update(uri, result);
            })
        );
        context.subscriptions.push(
            commands.registerCommand("adept.restartServer", async () => {
                await killServer();
                startServer(context);
            }),
        );

    }
}

async function killServer(): Promise<void> {
    await client.stop();
    server?.kill();
}

export function activate(context: ExtensionContext) {
    startServer(context);

}
export function deactivate(): Thenable<void> | undefined {
    return killServer();
}

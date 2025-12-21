import { type ChildProcess, spawn } from "child_process";
import type { ExtensionContext } from "vscode";
import { commands, window, workspace } from "vscode";
import type {
    LanguageClientOptions,
    ServerOptions,
} from "vscode-languageclient/node";

import { LanguageClient } from "vscode-languageclient/node";

let client: LanguageClient;
let server: ChildProcess | undefined;

function startServer() {
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

        client.start().then(() =>
            outputChannel.appendLine("started adept.")
        );
    }
}

async function killServer(): Promise<void> {
    await client.stop();
    server?.kill();
}

export function activate(context: ExtensionContext) {
    startServer();
    context.subscriptions.push(
        commands.registerCommand("adept.restartServer", async () => {
            await killServer();
            startServer();
        }),
    );
}
export function deactivate(): Thenable<void> | undefined {
    return killServer();
}

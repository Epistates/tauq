import * as path from "path";
import { workspace, ExtensionContext, window, commands, ProgressLocation } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";
import * as cp from "child_process";
import * as fs from "fs";

let client: LanguageClient;

export async function activate(context: ExtensionContext) {
  // Get the path to the language server executable
  const config = workspace.getConfiguration("tauq");
  let serverPath = config.get<string>("lsp.path") || "tauq-lsp";

  // Check if tauq-lsp exists
  const isInstalled = await new Promise<boolean>((resolve) => {
    cp.exec(`${serverPath} --version`, (err) => {
      resolve(!err);
    });
  });

  if (!isInstalled) {
    const selection = await window.showErrorMessage(
      `Tauq Language Server not found at '${serverPath}'. Would you like to install it via Cargo?`,
      "Install",
      "Cancel"
    );

    if (selection === "Install") {
      await window.withProgress(
        {
          location: ProgressLocation.Notification,
          title: "Installing Tauq Language Server...",
          cancellable: false,
        },
        async (progress) => {
          return new Promise<void>((resolve, reject) => {
            cp.exec("cargo install tauq --features lsp", (err, stdout, stderr) => {
              if (err) {
                window.showErrorMessage(`Installation failed: ${stderr || err.message}`);
                reject(err);
              } else {
                window.showInformationMessage("Tauq Language Server installed successfully!");
                resolve();
              }
            });
          });
        }
      );
    } else {
      window.showWarningMessage("Tauq features will be limited without the Language Server.");
      return;
    }
  }

  // Server options - run the tauq-lsp binary
  const serverOptions: ServerOptions = {
    run: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
    debug: {
      command: serverPath,
      transport: TransportKind.stdio,
    },
  };

  // Client options
  const clientOptions: LanguageClientOptions = {
    // Register for Tauq documents (.tqn and .tqq)
    documentSelector: [
      { scheme: "file", language: "tauq" },
      { scheme: "untitled", language: "tauq" },
    ],
    synchronize: {
      // Watch for changes to .tqn and .tqq files
      fileEvents: workspace.createFileSystemWatcher("**/*.{tqn,tqq}"),
    },
  };

  // Create the language client
  client = new LanguageClient(
    "tauq-lsp",
    "Tauq Language Server",
    serverOptions,
    clientOptions
  );

  // Start the client (also launches the server)
  client.start();

  window.showInformationMessage("Tauq Language Server activated");

  // Register commands
  context.subscriptions.push(
    commands.registerCommand("tauq.convertToJson", () => {
      const editor = window.activeTextEditor;
      if (!editor) return;
      const file = editor.document.fileName;
      cp.exec(`tauq --to-json "${file}"`, (err, stdout, stderr) => {
        if (err) {
          window.showErrorMessage(`Tauq Error: ${stderr || err.message}`);
          return;
        }
        workspace.openTextDocument({ content: stdout, language: "json" }).then(doc => {
          window.showTextDocument(doc);
        });
      });
    }),
    commands.registerCommand("tauq.convertFromJson", () => {
      const editor = window.activeTextEditor;
      if (!editor) return;
      const file = editor.document.fileName;
      cp.exec(`tauq --from-json "${file}"`, (err, stdout, stderr) => {
        if (err) {
          window.showErrorMessage(`Tauq Error: ${stderr || err.message}`);
          return;
        }
        workspace.openTextDocument({ content: stdout, language: "tauq" }).then(doc => {
          window.showTextDocument(doc);
        });
      });
    }),
    commands.registerCommand("tauq.minify", () => {
      const editor = window.activeTextEditor;
      if (!editor) return;
      const file = editor.document.fileName;
      cp.exec(`tauq --minify "${file}"`, (err, stdout, stderr) => {
        if (err) {
          window.showErrorMessage(`Tauq Error: ${stderr || err.message}`);
          return;
        }
        workspace.openTextDocument({ content: stdout, language: "tauq" }).then(doc => {
          window.showTextDocument(doc);
        });
      });
    })
  );
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

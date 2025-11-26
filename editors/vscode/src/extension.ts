import * as path from "path";
import { workspace, ExtensionContext, window } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient;

export function activate(context: ExtensionContext) {
  // Get the path to the language server executable
  const config = workspace.getConfiguration("tauq");
  const serverPath = config.get<string>("lsp.path") || "tauq-lsp";

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
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

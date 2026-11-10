import path from "node:path";
import * as vscode from "vscode";
import {
  DidChangeConfigurationNotification,
  LanguageClient,
  RevealOutputChannelOn,
} from "vscode-languageclient/node";
import {
  analysisOptions,
  scoopHome,
  serverCommand,
} from "./server-settings.ts";

interface Service {
  client: LanguageClient;
  folder: vscode.WorkspaceFolder | undefined;
}

export class ScoopLanguageServer {
  private readonly clients = new Map<string, Service>();
  private pending = Promise.resolve();
  private disposed = false;
  private watchers: vscode.Disposable[] = [];
  private watched = "";
  private readonly output = vscode.window.createOutputChannel("Scoop LSP", {
    log: true,
  });

  constructor(context: vscode.ExtensionContext) {
    context.subscriptions.push(
      this.output,
      vscode.commands.registerCommand("scoop.restartLanguageServer", () =>
        this.update(true),
      ),
      vscode.commands.registerCommand("scoop.showLanguageServerOutput", () =>
        this.output.show(),
      ),
      vscode.workspace.onDidGrantWorkspaceTrust(() => void this.update(true)),
      vscode.workspace.onDidChangeWorkspaceFolders(() => void this.update()),
      vscode.window.onDidChangeActiveTextEditor(() => {
        if (!vscode.workspace.workspaceFolders?.length) void this.update();
      }),
      vscode.workspace.onDidChangeConfiguration((event) => {
        if (event.affectsConfiguration("scoop"))
          void this.update(event.affectsConfiguration("scoop.server"));
      }),
    );
  }

  update(restart = false): Promise<void> {
    this.pending = this.pending
      .then(async () => {
        const folders: (vscode.WorkspaceFolder | undefined)[] = vscode.workspace
          .workspaceFolders?.length
          ? [...vscode.workspace.workspaceFolders]
          : [undefined];
        const active = vscode.window.activeTextEditor?.document.uri;
        const fallback =
          active?.scheme === "file"
            ? path.dirname(active.fsPath)
            : process.cwd();
        const desired = new Map(
          folders.map((folder) => [folder?.uri.fsPath ?? fallback, folder]),
        );
        for (const [cwd, service] of this.clients) {
          const settings = vscode.workspace.getConfiguration(
            "scoop",
            service.folder?.uri,
          );
          if (
            restart ||
            this.disposed ||
            !vscode.workspace.isTrusted ||
            !desired.has(cwd) ||
            !settings.get("server.enabled", true)
          ) {
            await this.stop(cwd);
          }
        }
        if (this.disposed) {
          this.clearWatchers();
          return;
        }
        this.watchToolchains([...desired.keys()]);
        if (!vscode.workspace.isTrusted) return;
        for (const [cwd, folder] of desired) {
          const settings = vscode.workspace.getConfiguration(
            "scoop",
            folder?.uri,
          );
          if (!settings.get("server.enabled", true)) continue;
          const existing = this.clients.get(cwd)?.client;
          if (existing) {
            if (existing.isRunning())
              await existing.sendNotification(
                DidChangeConfigurationNotification.type,
                { settings: { scoop: this.options(folder) } },
              );
            continue;
          }
          const command = await serverCommand(settings.get("server.path", ""));
          this.output.info(`Starting ${command} --stdio in ${cwd}`);
          const pattern = folder
            ? { baseUri: folder.uri.toString(), pattern: "**/*" }
            : "**/*";
          const client = new LanguageClient(
            "scoop",
            `Scoop Language Server${folder ? ` (${folder.name})` : ""}`,
            { command, args: ["--stdio"], options: { cwd } },
            {
              documentSelector: [
                { language: "scoop", scheme: "file", pattern },
                {
                  scheme: "file",
                  pattern: folder
                    ? {
                        baseUri: folder.uri.toString(),
                        pattern: "**/Cone.toml",
                      }
                    : "**/Cone.toml",
                },
                ...(folder === folders[0]
                  ? [{ language: "scoop", scheme: "untitled" }]
                  : []),
              ],
              workspaceFolder: folder,
              initializationOptions: () => this.options(folder),
              outputChannel: this.output,
              traceOutputChannel: this.output,
              revealOutputChannelOn: RevealOutputChannelOn.Never,
              initializationFailedHandler: () => false,
              middleware: { provideFoldingRanges: () => [] },
            },
          );
          this.clients.set(cwd, { client, folder });
          try {
            await client.start();
          } catch {
            await this.stop(cwd);
            this.output.info(
              "Check scoop.server.path, then run Scoop: Restart Language Server.",
            );
          }
        }
      })
      .catch((error: unknown) => {
        const message = error instanceof Error ? error.message : String(error);
        this.output.error(message);
        if (this.disposed) return;
        void vscode.window
          .showErrorMessage(
            `Scoop LSP: ${message}. Check scoop.server.path and the installed toolchain.`,
            "Show Output",
          )
          .then((action) => {
            if (action === "Show Output") this.output.show();
          });
      });
    return this.pending;
  }

  private options(folder: vscode.WorkspaceFolder | undefined) {
    return analysisOptions(
      vscode.workspace
        .getConfiguration(undefined, folder?.uri)
        .get("scoop", {}),
    );
  }

  private watchToolchains(directories: string[]) {
    const files = new Set([path.join(scoopHome(), "settings.toml")]);
    for (const directory of directories) {
      let current = directory;
      for (;;) {
        files.add(path.join(current, "scoop-toolchain.toml"));
        const parent = path.dirname(current);
        if (parent === current) break;
        current = parent;
      }
    }
    const key = [...files].sort().join("\n");
    if (key === this.watched) return;
    this.clearWatchers();
    this.watched = key;
    for (const file of files) {
      const watcher = vscode.workspace.createFileSystemWatcher(
        new vscode.RelativePattern(path.dirname(file), path.basename(file)),
      );
      const restart = () => void this.update(true);
      this.watchers.push(
        watcher,
        watcher.onDidChange(restart),
        watcher.onDidCreate(restart),
        watcher.onDidDelete(restart),
      );
    }
  }

  private clearWatchers() {
    for (const watcher of this.watchers) watcher.dispose();
    this.watchers = [];
    this.watched = "";
  }

  private async stop(cwd: string) {
    const service = this.clients.get(cwd);
    this.clients.delete(cwd);
    try {
      await service?.client.dispose();
    } catch (error) {
      this.output.warn(`Language server cleanup: ${String(error)}`);
    }
  }

  dispose(): Promise<void> {
    this.disposed = true;
    return this.update();
  }
}

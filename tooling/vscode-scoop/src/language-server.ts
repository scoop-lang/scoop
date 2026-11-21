import path from "node:path";
import * as vscode from "vscode";
import {
  DidChangeConfigurationNotification,
  LanguageClient,
  type Location as ProtocolLocation,
  type Position as ProtocolPosition,
  type WorkspaceEdit as ProtocolWorkspaceEdit,
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

interface SafeDeleteUsages {
  uri: string;
  version: number | null;
  position: ProtocolPosition;
  locations: ProtocolLocation[];
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
      vscode.commands.registerCommand("scoop.goToSuperDeclaration", () =>
        this.goToLocations(
          "scoop/superDeclarations",
          "scoopSuperDeclarations",
          "Go to Super Declaration",
          "No overridden declaration with available source.",
        ),
      ),
      vscode.commands.registerCommand("scoop.goToRelatedTests", () =>
        this.goToLocations(
          "scoop/relatedTests",
          "scoopRelatedTests",
          "Go to Related Tests",
          "No test usage in the current configuration.",
        ),
      ),
      vscode.commands.registerCommand("scoop.showMemberHierarchy", () =>
        this.showMemberHierarchy(),
      ),
      vscode.commands.registerCommand("scoop.moveDeclaration", () =>
        this.moveDeclaration(),
      ),
      vscode.commands.registerCommand("scoop.changeSignature", () =>
        this.changeSignature(),
      ),
      vscode.commands.registerCommand(
        "scoop.showSafeDeleteUsages",
        (usages: SafeDeleteUsages) => this.showSafeDeleteUsages(usages),
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

  private clientFor(uri: vscode.Uri) {
    const folder = vscode.workspace.getWorkspaceFolder(uri);
    return (
      folder
        ? this.clients.get(folder.uri.fsPath)
        : this.clients.values().next().value
    )?.client;
  }

  private async showSafeDeleteUsages(usages: SafeDeleteUsages) {
    const editor = vscode.window.activeTextEditor;
    if (
      editor?.document.uri.toString() !== usages.uri ||
      editor.document.version !== usages.version
    )
      return;
    const uri = editor.document.uri;
    const client = this.clientFor(uri);
    if (!client?.isRunning()) return;
    await vscode.commands.executeCommand(
      "editor.action.goToLocations",
      uri,
      client.protocol2CodeConverter.asPosition(usages.position),
      usages.locations.map((location) =>
        client.protocol2CodeConverter.asLocation(location),
      ),
      "peek",
      "No usage with available source.",
    );
  }

  private async goToLocations(
    method: string,
    capability: string,
    label: string,
    empty: string,
  ) {
    const editor = vscode.window.activeTextEditor;
    if (editor?.document.languageId !== "scoop") return;
    const { document, selection } = editor;
    const client = this.clientFor(document.uri);
    if (
      !client?.isRunning() ||
      !client.initializeResult?.capabilities.experimental?.[capability]
    ) {
      await vscode.window.showInformationMessage(
        `The active Scoop language server does not provide ${label}.`,
      );
      return;
    }
    const version = document.version;
    const position = selection.active;
    const locations = await client.sendRequest<ProtocolLocation[]>(
      method,
      client.code2ProtocolConverter.asTextDocumentPositionParams(
        document,
        position,
      ),
    );
    if (
      editor.document.version !== version ||
      vscode.window.activeTextEditor !== editor
    )
      return;
    await vscode.commands.executeCommand(
      "editor.action.goToLocations",
      document.uri,
      position,
      locations.map((location) =>
        client.protocol2CodeConverter.asLocation(location),
      ),
      "peek",
      empty,
    );
  }

  private async showMemberHierarchy() {
    const editor = vscode.window.activeTextEditor;
    if (editor?.document.languageId !== "scoop") return;
    const { document, selection } = editor;
    const client = this.clientFor(document.uri);
    if (
      !client?.isRunning() ||
      !client.initializeResult?.capabilities.experimental?.scoopMemberHierarchy
    ) {
      await vscode.window.showInformationMessage(
        "The active Scoop language server does not provide Member Hierarchy.",
      );
      return;
    }
    const version = document.version;
    const result = await client.sendRequest<{
      parents: ProtocolLocation[];
      children: ProtocolLocation[];
    }>(
      "scoop/memberHierarchy",
      client.code2ProtocolConverter.asTextDocumentPositionParams(
        document,
        selection.active,
      ),
    );
    const choices = [
      { label: "Direct Super Declarations", locations: result.parents },
      { label: "Direct Overrides", locations: result.children },
    ].filter((choice) => choice.locations.length > 0);
    if (!choices.length) {
      await vscode.window.showInformationMessage(
        "No member hierarchy with available source.",
      );
      return;
    }
    const choice =
      choices.length === 1
        ? choices[0]
        : await vscode.window.showQuickPick(choices, {
            placeHolder: "Choose a direction in the member hierarchy",
          });
    if (
      !choice ||
      document.version !== version ||
      vscode.window.activeTextEditor !== editor
    )
      return;
    await vscode.commands.executeCommand(
      "editor.action.goToLocations",
      document.uri,
      selection.active,
      choice.locations.map((location) =>
        client.protocol2CodeConverter.asLocation(location),
      ),
      "peek",
    );
  }

  private async moveDeclaration() {
    const editor = vscode.window.activeTextEditor;
    if (editor?.document.languageId !== "scoop") return;
    const { document, selection } = editor;
    const client = this.clientFor(document.uri);
    if (
      !client?.isRunning() ||
      !client.initializeResult?.capabilities.experimental?.scoopMoveDeclaration
    )
      return;
    const version = document.version;
    const [target] =
      (await vscode.window.showOpenDialog({
        title: "Move Declaration to a Scoop File",
        openLabel: "Choose Destination",
        canSelectMany: false,
        filters: { Scoop: ["scoop"] },
        defaultUri: vscode.workspace.getWorkspaceFolder(document.uri)?.uri,
      })) ?? [];
    if (!target || document.version !== version) return;
    const edit = await client.sendRequest<ProtocolWorkspaceEdit | null>(
      "scoop/moveDeclaration",
      {
        ...client.code2ProtocolConverter.asTextDocumentPositionParams(
          document,
          selection.active,
        ),
        targetUri: client.code2ProtocolConverter.asUri(target),
      },
    );
    if (document.version === version)
      await this.applyRefactoring(client, edit, "Move Declaration");
  }

  private async changeSignature() {
    const editor = vscode.window.activeTextEditor;
    if (editor?.document.languageId !== "scoop") return;
    const { document, selection } = editor;
    const client = this.clientFor(document.uri);
    if (
      !client?.isRunning() ||
      !client.initializeResult?.capabilities.experimental?.scoopChangeSignature
    )
      return;
    const version = document.version;
    const position = client.code2ProtocolConverter.asTextDocumentPositionParams(
      document,
      selection.active,
    );
    const plan = await client.sendRequest<{
      parameters: string[];
      revision: number;
    } | null>("scoop/prepareChangeSignature", position);
    if (!plan) {
      await vscode.window.showInformationMessage(
        "Select a private function declaration with two or more parameters.",
      );
      return;
    }
    const parseOrder = (input: string) =>
      input.split(",").map((number) => Number(number.trim()) - 1);
    const value = await vscode.window.showInputBox({
      title: "Change Parameter Order",
      prompt: plan.parameters
        .map((name, index) => `${index + 1}. ${name}`)
        .join(", "),
      value: plan.parameters.map((_, index) => index + 1).join(", "),
      validateInput: (input) => {
        const order = parseOrder(input);
        return order.length === plan.parameters.length &&
          new Set(order).size === order.length &&
          order.every(
            (index) =>
              Number.isInteger(index) && index >= 0 && index < order.length,
          )
          ? undefined
          : "Enter each parameter number once.";
      },
    });
    if (value === undefined || document.version !== version) return;
    const edit = await client.sendRequest<ProtocolWorkspaceEdit | null>(
      "scoop/changeSignature",
      { ...position, revision: plan.revision, order: parseOrder(value) },
    );
    if (document.version === version)
      await this.applyRefactoring(client, edit, "Change Signature");
  }

  private async applyRefactoring(
    client: LanguageClient,
    edit: ProtocolWorkspaceEdit | null,
    label: string,
  ) {
    if (!edit) {
      await vscode.window.showInformationMessage(
        `${label} is unavailable for the selected declaration.`,
      );
      return;
    }
    for (const change of edit.documentChanges ?? []) {
      if (!("textDocument" in change) || change.textDocument.version === null)
        continue;
      const uri = client.protocol2CodeConverter.asUri(change.textDocument.uri);
      const document = vscode.workspace.textDocuments.find(
        (candidate) => candidate.uri.toString() === uri.toString(),
      );
      if (document?.version !== change.textDocument.version) return;
    }
    const converted = await client.protocol2CodeConverter.asWorkspaceEdit(edit);
    if (!converted?.size) return;
    const preview = new vscode.WorkspaceEdit();
    for (const [uri, edits] of converted.entries()) {
      preview.set(
        uri,
        edits.map(
          (
            sourceEdit,
          ): [vscode.TextEdit, vscode.WorkspaceEditEntryMetadata] => [
            sourceEdit,
            { label, needsConfirmation: true },
          ],
        ),
      );
    }
    await vscode.workspace.applyEdit(preview, { isRefactoring: true });
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

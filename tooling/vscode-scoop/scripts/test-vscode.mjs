import {
  access,
  mkdir,
  mkdtemp,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { runTests } from "@vscode/test-electron";
import { build } from "esbuild";

const root = fileURLToPath(new URL("../", import.meta.url));
const repository = path.resolve(root, "../..");
const server = path.resolve(
  process.env.SCOOP_TEST_LSP ??
    path.join(
      repository,
      "target/release",
      process.platform === "win32" ? "scoop-lsp.exe" : "scoop-lsp",
    ),
);
const sysroot = path.resolve(
  process.env.SCOOP_SYSROOT ?? path.join(repository, "sysroot"),
);
const compiler = path.resolve(
  process.env.SCOOP_TEST_COMPILER ??
    path.join(
      path.dirname(server),
      process.platform === "win32" ? "scoop.exe" : "scoop",
    ),
);
await access(server);
await access(compiler);
await access(path.join(sysroot, "lib"));
const extensionTestsPath = path.join(root, "out/test/integration.cjs");
await build({
  entryPoints: [path.join(root, "test/host/integration.ts")],
  outfile: extensionTestsPath,
  bundle: true,
  platform: "node",
  format: "cjs",
  target: "node20",
  external: ["vscode"],
});

const scratch = path.join(repository, "tmp");
await mkdir(scratch, { recursive: true });
const temporary = await mkdtemp(path.join(await realpath(scratch), "v-"));
const workspace = path.join(temporary, "workspace");
await mkdir(workspace, { recursive: true });
const workspaceFile = path.join(temporary, "test.code-workspace");
await writeFile(
  workspaceFile,
  JSON.stringify({
    folders: [{ path: workspace }],
    settings: {
      "scoop.server.enabled": false,
      "scoop.server.path": server,
      "scoop.debug.compilerPath": compiler,
      "scoop.sysroot": sysroot,
      "scoop.trace.server": "verbose",
    },
  }),
);
let succeeded = false;
try {
  await runTests({
    extensionDevelopmentPath: root,
    extensionTestsPath,
    vscodeExecutablePath: process.env.VSCODE_EXECUTABLE_PATH,
    version: process.env.VSCODE_VERSION,
    launchArgs: [
      workspaceFile,
      "--user-data-dir",
      path.join(temporary, "p"),
      "--extensions-dir",
      path.join(temporary, "e"),
      "--disable-extensions",
      "--log",
      "trace",
      "--disable-workspace-trust",
      "--skip-welcome",
      "--skip-release-notes",
    ],
  });
  succeeded = true;
} finally {
  if (succeeded) await rm(temporary, { recursive: true, force: true });
  else console.error(`Extension host test logs and workspace: ${temporary}`);
}

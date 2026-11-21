import * as vscode from "vscode";

type ParameterEdit =
  | { kind: "addDefault"; name: string; type: string; expression: string }
  | { kind: "removeUnused"; parameter: number }
  | { kind: "setDefault"; parameter: number; expression: string | null };

type SignatureChange = { order: number[] } | { edit: ParameterEdit };
type Operation = "order" | ParameterEdit["kind"];

export async function chooseSignatureChange(
  parameters: string[],
): Promise<SignatureChange | undefined> {
  const operations: (vscode.QuickPickItem & { operation: Operation })[] = [];
  if (parameters.length > 1)
    operations.push({ label: "Reorder Parameters", operation: "order" });
  operations.push({ label: "Add Defaulted Parameter", operation: "addDefault" });
  if (parameters.length > 0) {
    operations.push(
      { label: "Remove Unused Parameter", operation: "removeUnused" },
      { label: "Set or Clear Default Value", operation: "setDefault" },
    );
  }
  const choice = await vscode.window.showQuickPick(operations, {
    title: "Change Signature",
  });
  switch (choice?.operation) {
    case "order":
      return chooseOrder(parameters);
    case "addDefault": {
      const name = await requiredInput("New Parameter Name");
      if (name === undefined) return;
      const type = await requiredInput("New Parameter Type");
      if (type === undefined) return;
      const expression = await requiredInput("New Parameter Default Expression");
      if (expression === undefined) return;
      return { edit: { kind: "addDefault", name, type, expression } };
    }
    case "removeUnused":
    case "setDefault": {
      const items = parameters.map((label, parameter) => ({ label, parameter }));
      const selected = await vscode.window.showQuickPick(items, {
        title: "Select Parameter",
      });
      if (!selected) return;
      const { parameter } = selected;
      if (choice.operation === "removeUnused")
        return { edit: { kind: "removeUnused", parameter } };
      const expression = await vscode.window.showInputBox({
        title: "Default Expression",
        prompt: "Leave empty to clear the default value.",
      });
      if (expression === undefined) return;
      return {
        edit: { kind: "setDefault", parameter, expression: expression.trim() || null },
      };
    }
  }
}

async function requiredInput(title: string): Promise<string | undefined> {
  const value = await vscode.window.showInputBox({
    title,
    validateInput: (input) => (input.trim() ? undefined : "Enter a value."),
  });
  return value?.trim();
}

async function chooseOrder(
  parameters: string[],
): Promise<SignatureChange | undefined> {
  const parseOrder = (input: string) =>
    input.split(",").map((number) => Number(number.trim()) - 1);
  const value = await vscode.window.showInputBox({
    title: "Change Parameter Order",
    prompt: parameters.map((name, index) => `${index + 1}. ${name}`).join(", "),
    value: parameters.map((_, index) => index + 1).join(", "),
    validateInput: (input) => {
      const order = parseOrder(input);
      return order.length === parameters.length &&
        new Set(order).size === order.length &&
        order.every(
          (index) =>
            Number.isInteger(index) && index >= 0 && index < order.length,
        )
        ? undefined
        : "Enter each parameter number once.";
    },
  });
  return value === undefined ? undefined : { order: parseOrder(value) };
}

type CopyResult = "copied" | "unavailable";

const MESSAGES: Record<CopyResult, string> = {
  copied: "Copied to clipboard.",
  unavailable: "Clipboard unavailable. Select the text to copy it.",
};

async function writeClipboard(text: string): Promise<CopyResult> {
  try {
    await navigator.clipboard.writeText(text);
  } catch {
    return "unavailable";
  }
  return "copied";
}

export function resetCopy(button: HTMLButtonElement, status: HTMLElement): void {
  delete button.dataset.copied;
  status.textContent = "";
  status.classList.add("visually-hidden");
}

export async function copyText(button: HTMLButtonElement, status: HTMLElement, readText: () => string): Promise<void> {
  const text = readText();
  if (text === "") return;
  resetCopy(button, status);
  const result = await writeClipboard(text);
  if (!button.isConnected || text !== readText()) return;
  button.toggleAttribute("data-copied", result === "copied");
  status.classList.toggle("visually-hidden", result === "copied");
  status.textContent = MESSAGES[result];
}

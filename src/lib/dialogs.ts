import { isTauri } from "./api";

export async function pickFolder(title: string): Promise<string | null> {
  if (!isTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({ directory: true, multiple: false, title });
  return typeof result === "string" ? result : null;
}

export async function pickSave(title: string, defaultPath: string, filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  if (!isTauri) return null;
  const { save } = await import("@tauri-apps/plugin-dialog");
  const result = await save({ title, defaultPath, filters });
  return result ?? null;
}

export async function pickFile(title: string, filters?: { name: string; extensions: string[] }[]): Promise<string | null> {
  if (!isTauri) return null;
  const { open } = await import("@tauri-apps/plugin-dialog");
  const result = await open({ directory: false, multiple: false, title, filters });
  return typeof result === "string" ? result : null;
}

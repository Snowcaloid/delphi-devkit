import { dirname, basename, join } from "path/posix";
import { Uri, workspace } from "vscode";
import fs from "fs";

export function fileExists(filePath: string | Uri | undefined | null): boolean {
  try { 
    return !!filePath && !!(fs.statSync(filePath instanceof Uri ? filePath.fsPath : filePath));
  } catch {
    return false;
  }
}

export function removeBOM(content: string): string {
  if (content.charCodeAt(0) === 0xFEFF) {
    return content.substring(1);
  }
  return content;
}
export async function findIniFromExecutable(executableUri?: string): Promise<string | undefined> {
  if (!executableUri) { return undefined; }
  try {
    const executableDir = dirname(executableUri);
    const executableName = basename(executableUri).replace(/\.[^/.]+$/, "");
    const iniPath = join(executableDir, `${executableName}.ini`);

    try {
      await workspace.fs.stat(Uri.file(iniPath));
      return iniPath;
    } catch {
      return undefined;
    }
  } catch (error) {
    console.error('Failed to find INI from executable:', error);
    return undefined;
  }
}

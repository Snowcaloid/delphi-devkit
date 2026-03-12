import { basename, extname } from 'path';
import { Uri, workspace, window } from 'vscode';

export async function fileExists(filePath: string | Uri | undefined | null): Promise<boolean> {
  if (!filePath) return false;
  try {
    const uri = filePath instanceof Uri ? filePath : Uri.file(filePath);
    await workspace.fs.stat(uri);
    return true;
  } catch {
    return false;
  }
}

export function basenameNoExt(filePath: string | Uri): string {
  if (filePath instanceof Uri) filePath = filePath.fsPath;

  return basename(filePath, extname(filePath));
}

export function assertError(condition: any, message: string): boolean {
  return !!condition || (window.showErrorMessage(message), false);
}

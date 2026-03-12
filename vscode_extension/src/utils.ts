import { dirname, basename, join, extname } from 'path';
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

export function removeBOM(content: string): string {
  if (content.charCodeAt(0) === 0xfeff) return content.substring(1);

  return content;
}

export async function findIniFromExecutable(executableUri?: string): Promise<Uri | undefined> {
  if (!executableUri) return undefined;
  try {
    const executableDir = dirname(executableUri);
    const executableName = basenameNoExt(executableUri);
    const iniPath = join(executableDir, `${executableName}.ini`);
    const ini = Uri.file(iniPath);

    try {
      await workspace.fs.stat(ini);
      return ini;
    } catch {
      return undefined;
    }
  } catch (error) {
    console.error('Failed to find INI from executable:', error);
    return undefined;
  }
}

export function basenameNoExt(filePath: string | Uri): string {
  if (filePath instanceof Uri) filePath = filePath.fsPath;

  return basename(filePath, extname(filePath));
}

function assert(condition: boolean, message: string, callback: (message: string) => any): boolean {
  if (condition) return true;

  callback(message);
  return false;
}

export function assertError(condition: any, message: string): boolean {
  return assert(condition, message, window.showErrorMessage);
}

export function assertWarning(condition: any, message: string): boolean {
  return assert(condition, message, window.showWarningMessage);
}

export function assertInfo(condition: any, message: string): boolean {
  return assert(condition, message, window.showInformationMessage);
}

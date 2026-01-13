import { commands, env, Uri, window, Disposable } from "vscode";
import { COMMANDS } from "./constants";
import { join } from "path";
import { promises as fs } from 'fs';
import { Runtime } from "./runtime";
import { ExtensionDataExport } from "./types";
import { assertError } from "./utils";

export class GeneralCommands {
  public static get registers(): Disposable[] {
    return [
      commands.registerCommand(COMMANDS.EXPORT_CONFIGURATION, this.exportConfiguration.bind(this)),
      commands.registerCommand(COMMANDS.IMPORT_CONFIGURATION, this.importConfiguration.bind(this))
    ];
  }

  private static async exportConfiguration(): Promise<void> {
    const fileUri = await window.showSaveDialog({
      saveLabel: 'Export DDK',
      title: 'Export DDK Configuration',
      filters: {
        'DDK JSON files': ['ddk.json'],
        'All files': ['*']
      },
      defaultUri: Uri.file(join(env.appRoot, 'configuration.ddk.json'))
    });
    if (!fileUri) return;
    const data = Runtime.projectsData;
    if (!assertError(data, 'No configuration data to export.')) return;
    const compilers = Runtime.compilerConfigurations;
    if (!assertError(compilers, 'No compiler configuration data to export.')) return;
    try {
      const fileData = new ExtensionDataExport.FileContent(data!, compilers!);
      await fs.writeFile(fileUri.fsPath, JSON.stringify(fileData, null, 2), 'utf8');
      window.showInformationMessage('Configuration exported successfully.');
    } catch (error) {
      window.showErrorMessage(`Failed to export configuration: ${error}`);
    }
  }

  private static async importConfigurationV2_0(data: ExtensionDataExport.FileContent): Promise<void> {
    await Runtime.client.compilersOverride(data.compilers);
    await Runtime.client.projectsDataOverride(data.projectsData);
  }

  private static async importConfiguration(): Promise<void> {
    const fileUri = (
      await window.showOpenDialog({
        canSelectMany: false,
        title: 'Import DDK Configuration',
        canSelectFolders: false,
        canSelectFiles: true,
        openLabel: 'Import',
        filters: {
          'DDK JSON files': ['ddk.json'],
          'All files': ['*']
        }
      })
    )?.[0];
    if (!fileUri) return;
    try {
      const content = await fs.readFile(fileUri.fsPath, 'utf8');
      const data = JSON.parse(content) as ExtensionDataExport.FileContent;
      if (data) {
        switch (data.version as ExtensionDataExport.Version) {
          case ExtensionDataExport.Version.V2_0:
            await this.importConfigurationV2_0(data);
            break;
          default:
            window.showErrorMessage(`Unsupported configuration version: ${data.version}`);
            return;
        }
        await Runtime.projects.workspacesTreeView.refresh();
        await Runtime.projects.groupProjectTreeView.refresh();
        await Runtime.projects.compilerStatusBarItem.updateDisplay();
        window.showInformationMessage('Configuration imported successfully.');
      } else window.showErrorMessage('Invalid configuration file.');
    } catch (error) {
      window.showErrorMessage(`Failed to import configuration: ${error}`);
    }
  }
}
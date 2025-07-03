import { commands, window, Uri, env } from 'vscode';
import { DprTreeItem } from '../DprTreeItem';
import { DprFile } from '../DprFile';
import { DprojFile } from '../DprojFile';
import { ExecutableFile } from '../ExecutableFile';
import { DprUtils } from '../utils';
import { Compiler } from './Compiler';

/**
 * Context menu commands for DPR Explorer tree items
 */
export class DprContextMenuCommands {
  /**
   * Register all context menu commands
   */
  static registerCommands() {
    return [
      // Build commands
      commands.registerCommand('delphi-utils.compileDpr', DprContextMenuCommands.compileDpr),
      commands.registerCommand('delphi-utils.recreateDpr', DprContextMenuCommands.recreateDpr),

      // File commands
      commands.registerCommand('delphi-utils.showInExplorer', DprContextMenuCommands.showInExplorer),
      commands.registerCommand('delphi-utils.openInFileExplorer', DprContextMenuCommands.openInFileExplorer),

      // Run commands
      commands.registerCommand('delphi-utils.runExecutable', DprContextMenuCommands.runExecutable),

      // Compiler configuration
      commands.registerCommand('delphi-utils.selectCompilerConfiguration', DprContextMenuCommands.selectCompilerConfiguration)
    ];
  }

  /**
   * Compile DPR project - works for any item type by finding the associated DPR/DPROJ
   */
  private static async compileDpr(item: DprTreeItem): Promise<void> {
    const dprojUri = await DprContextMenuCommands.findDprojUri(item);
    if (dprojUri) {
      await Compiler.compile(dprojUri, false);
    }
  }

  /**
   * Recreate DPR project - works for any item type by finding the associated DPR/DPROJ
   */
  private static async recreateDpr(item: DprTreeItem): Promise<void> {
    const dprojUri = await DprContextMenuCommands.findDprojUri(item);
    if (dprojUri) {
      await Compiler.compile(dprojUri, true);
    }
  }

  /**
   * Helper method to find the DPROJ URI from any tree item
   */
  private static async findDprojUri(item: DprTreeItem): Promise<Uri | null> {
    if (item instanceof DprojFile) {
      return item.resourceUri;
    } else if (item instanceof DprFile) {
      // If DprFile has associated dproj, use it; otherwise try to find it
      if (item.dproj) {
        return item.dproj;
      } else {
        return await DprUtils.findDprojFromDpr(item.resourceUri);
      }
    } else if (item instanceof ExecutableFile) {
      // For executable files, find the associated project files
      const projectFiles = await DprUtils.findProjectFromExecutable(item.resourceUri);
      if (projectFiles.dproj) {
        return projectFiles.dproj;
      } else {
        window.showWarningMessage(`No DPROJ file found for executable: ${item.label}`);
        return null;
      }
    } else {
      window.showWarningMessage(`Cannot compile unknown file type: ${item.label}`);
      return null;
    }
  }

  /**
   * Select compiler configuration
   */
  private static async selectCompilerConfiguration(): Promise<void> {
    const configurations = Compiler.getAvailableConfigurations();

    if (configurations.length === 0) {
      window.showErrorMessage('No compiler configurations found. Please configure Delphi compiler settings.');
      return;
    }

    const items = configurations.map(config => ({
      label: config.name,
      description: config.rsVarsPath,
      detail: `MSBuild: ${config.msBuildPath}`
    }));

    const selected = await window.showQuickPick(items, {
      placeHolder: 'Select Delphi Compiler Configuration',
      matchOnDescription: true,
      matchOnDetail: true
    });

    if (selected) {
      await Compiler.setCurrentConfiguration(selected.label);
    }
  }

  /**
   * Show file in VS Code explorer
   */
  private static async showInExplorer(item: DprTreeItem): Promise<void> {
    try {
      // Focus the file in VS Code explorer
      await commands.executeCommand('revealInExplorer', item.resourceUri);
    } catch (error) {
      window.showErrorMessage(`Failed to show in explorer: ${error}`);
    }
  }

  /**
   * Open containing folder in system file explorer
   */
  private static async openInFileExplorer(item: DprTreeItem): Promise<void> {
    try {
      // Open the containing folder in system file explorer
      const folderUri = Uri.file(item.resourceUri.fsPath.substring(0, item.resourceUri.fsPath.lastIndexOf('\\')));
      await env.openExternal(folderUri);
    } catch (error) {
      window.showErrorMessage(`Failed to open in file explorer: ${error}`);
    }
  }

  /**
   * Run executable file - works for any item type by finding the associated executable
   */
  private static async runExecutable(item: DprTreeItem): Promise<void> {
    try {
      let executableUri: Uri | undefined;

      if (item instanceof ExecutableFile) {
        // Direct executable file
        executableUri = item.resourceUri;
      } else if (item instanceof DprFile) {
        // DPR file - use its associated executable
        executableUri = item.executable;
      } else if (item instanceof DprojFile) {
        // DPROJ file - find the executable by parsing the dproj
        const foundExecutable = await DprUtils.findExecutableFromDproj(item.resourceUri);
        executableUri = foundExecutable || undefined;
      }

      if (executableUri) {
        await env.openExternal(executableUri);
        window.showInformationMessage(`Running: ${executableUri.fsPath}`);
      } else {
        window.showWarningMessage(`No executable found for: ${item.label}`);
      }
    } catch (error) {
      window.showErrorMessage(`Failed to run executable: ${error}`);
    }
  }
}

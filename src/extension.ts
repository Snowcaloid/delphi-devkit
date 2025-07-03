import { ExtensionContext, commands, languages, window, Uri, env } from 'vscode';
import { dfmSwap } from './dfmSwap/command';
import { DfmLanguageProvider } from './dfmLanguageSupport/provider';
import { DprExplorerProvider, DprContextMenuCommands, CompilerStatusBar, Compiler } from './dprExplorer';

export function activate(context: ExtensionContext): void {
  const swapCommand = commands.registerCommand('delphi-utils.swapToDfmPas', dfmSwap);
  const definitionProvider = languages.registerDefinitionProvider(
    { language: 'delphi-dfm', scheme: 'file' }, new DfmLanguageProvider());

  // Register DPR Explorer
  const dprExplorerProvider = new DprExplorerProvider();
  const dprTreeView = window.createTreeView('dprExplorer', {
    treeDataProvider: dprExplorerProvider
  });

  const refreshDprCommand = commands.registerCommand('delphi-utils.refreshDprExplorer', () => {
    dprExplorerProvider.refresh();
  });

  const launchExecutableCommand = commands.registerCommand('delphi-utils.launchExecutable', async (uri: Uri) => {
    try {
      // Use the system's default application handler to launch the executable
      await env.openExternal(uri);
    } catch (error) {
      window.showErrorMessage(`Failed to launch executable: ${error}`);
    }
  });

  // Register DPR Explorer context menu commands
  const contextMenuCommands = DprContextMenuCommands.registerCommands();

  // Initialize compiler status bar
  const compilerStatusBar = CompilerStatusBar.initialize();
  const compilerStatusBarCommands = CompilerStatusBar.registerCommands();

  context.subscriptions.push(
    swapCommand,
    definitionProvider,
    dprTreeView,
    refreshDprCommand,
    launchExecutableCommand,
    ...contextMenuCommands,
    ...compilerStatusBarCommands,
    compilerStatusBar
  );
}

export function deactivate(): void {
  // Clean up compiler terminal
  Compiler.dispose();
}

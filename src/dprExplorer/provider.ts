import { TreeItem, TreeDataProvider, TreeItemCollapsibleState, EventEmitter, Event, Uri, workspace, RelativePattern, ConfigurationChangeEvent } from 'vscode';
import { basename, dirname, join } from 'path';
import { minimatch } from 'minimatch';
import { promises as fs } from 'fs';
import { DOMParser } from '@xmldom/xmldom';
import { DprTreeItem } from './DprTreeItem';
import { DprFile } from './DprFile';
import { DprojFile } from './DprojFile';
import { ExecutableFile } from './ExecutableFile';

export class DprExplorerProvider implements TreeDataProvider<DprTreeItem> {
  private _onDidChangeTreeData: EventEmitter<DprTreeItem | undefined | null | void> = new EventEmitter<DprTreeItem | undefined | null | void>();
  readonly onDidChangeTreeData: Event<DprTreeItem | undefined | null | void> = this._onDidChangeTreeData.event;
  private configFileName = 'delphi-utils-dpr-list.json';

  constructor() {
    // Watch for file system changes to refresh the tree (case-insensitive patterns)
    const dprWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Rr]');
    const dprojWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Rr][Oo][Jj]');

    [dprWatcher, dprojWatcher].forEach(watcher => {
      watcher.onDidCreate(() => {
        this.refresh();
        this.saveDprListToConfig();
      });
      watcher.onDidDelete(() => {
        this.refresh();
        this.saveDprListToConfig();
      });
      watcher.onDidChange(() => this.refresh());
    });

    // Watch for configuration changes
    workspace.onDidChangeConfiguration((event: ConfigurationChangeEvent) => {
      if (event.affectsConfiguration('delphi-utils.dprExplorer.excludePatterns')) {
        this.refresh();
        this.saveDprListToConfig();
      }
    });
  }

  private async getConfigFilePath(): Promise<string | null> {
    if (!workspace.workspaceFolders || workspace.workspaceFolders.length === 0) {
      return null;
    }

    const workspaceRoot = workspace.workspaceFolders[0].uri.fsPath;
    const vscodeDir = join(workspaceRoot, '.vscode');

    // Ensure .vscode directory exists
    try {
      await fs.access(vscodeDir);
    } catch {
      await fs.mkdir(vscodeDir, { recursive: true });
    }

    return join(vscodeDir, this.configFileName);
  }

  private async saveDprListToConfig(): Promise<void> {
    const configPath = await this.getConfigFilePath();
    if (!configPath) {
      return;
    }

    try {
      const dprFiles = await this.getAllDprFiles();
      const configData = {
        lastUpdated: new Date().toISOString(),
        dprFiles: dprFiles.map(file => ({
          name: file.label,
          path: workspace.asRelativePath(file.resourceUri),
          absolutePath: file.resourceUri.fsPath,
          hasDproj: !!file.dproj,
          dprojPath: file.dproj ? workspace.asRelativePath(file.dproj) : undefined,
          dprojAbsolutePath: file.dproj?.fsPath,
          hasExecutable: !!file.executable,
          executablePath: file.executable ? workspace.asRelativePath(file.executable) : undefined,
          executableAbsolutePath: file.executable?.fsPath
        }))
      };

      await fs.writeFile(configPath, JSON.stringify(configData, null, 2), 'utf8');
    } catch (error) {
      console.error('Failed to save DPR list to config:', error);
    }
  }

  private async loadDprListFromConfig(): Promise<any> {
    const configPath = await this.getConfigFilePath();
    if (!configPath) {
      return null;
    }

    try {
      const configContent = await fs.readFile(configPath, 'utf8');
      return JSON.parse(configContent);
    } catch {
      // Config file doesn't exist or is invalid, return null
      return null;
    }
  }

  private async getAllDprFiles(): Promise<DprFile[]> {
    if (!workspace.workspaceFolders) {
      return [];
    }

    const dprFiles: DprFile[] = [];
    const dprojFiles: Uri[] = [];

    // Get exclude patterns from configuration
    const config = workspace.getConfiguration('delphi-utils.dprExplorer');
    const excludePatterns: string[] = config.get('excludePatterns', []);

    for (const folder of workspace.workspaceFolders) {
      // Search for DPR files with case-insensitive pattern
      const dprPattern = new RelativePattern(folder, '**/*.[Dd][Pp][Rr]');
      const dprFilesFound = await workspace.findFiles(dprPattern);

      // Search for DPROJ files with case-insensitive pattern
      const dprojPattern = new RelativePattern(folder, '**/*.[Dd][Pp][Rr][Oo][Jj]');
      const dprojFilesFound = await workspace.findFiles(dprojPattern);

      // Collect DPROJ files
      for (const file of dprojFilesFound) {
        const relativePath = workspace.asRelativePath(file, false);
        const shouldExclude = excludePatterns.some(pattern =>
          minimatch(relativePath, pattern, { matchBase: true })
        );
        if (!shouldExclude) {
          dprojFiles.push(file);
        }
      }

      // Process DPR files
      for (const file of dprFilesFound) {
        const relativePath = workspace.asRelativePath(file, false);

        // Check if file should be excluded based on patterns
        const shouldExclude = excludePatterns.some(pattern =>
          minimatch(relativePath, pattern, { matchBase: true })
        );

        if (!shouldExclude) {
          const fileName = basename(file.fsPath);
          const fileNameWithoutExt = fileName.replace(/\.[^/.]+$/, "");

          // Look for corresponding DPROJ file
          const correspondingDproj = dprojFiles.find(dprojFile => {
            const dprojName = basename(dprojFile.fsPath).replace(/\.[^/.]+$/, "");
            return dprojName.toLowerCase() === fileNameWithoutExt.toLowerCase() &&
                   dirname(dprojFile.fsPath) === dirname(file.fsPath);
          });

          const dprFile = new DprFile(
            fileName,
            file,
            TreeItemCollapsibleState.None  // Will be updated below if has children
          );

          if (correspondingDproj) {
            dprFile.dproj = correspondingDproj;

            // Try to parse executable path from DPROJ
            try {
              const executableUri = await this.parseExecutableFromDproj(correspondingDproj);
              if (executableUri) {
                dprFile.executable = executableUri;
              }
            } catch (error) {
              console.error('Failed to parse executable from DPROJ:', error);
            }
          }

          // Update collapsible state based on whether it has children
          if (dprFile.dproj || dprFile.executable) {
            dprFile.collapsibleState = TreeItemCollapsibleState.Collapsed;
          }

          dprFiles.push(dprFile);
        }
      }
    }

    return dprFiles;
  }

  private async loadDprFilesFromConfig(): Promise<DprFile[] | null> {
    const configData = await this.loadDprListFromConfig();
    if (!configData || !configData.dprFiles) {
      return null;
    }

    const dprFiles: DprFile[] = [];

    for (const fileData of configData.dprFiles) {
      try {
        // Verify the DPR file still exists
        const dprUri = Uri.file(fileData.absolutePath);
        await workspace.fs.stat(dprUri);

        // Create DPR file object
        const hasChildren = fileData.hasDproj || fileData.hasExecutable;
        const collapsibleState = hasChildren ? TreeItemCollapsibleState.Collapsed : TreeItemCollapsibleState.None;
        const dprFile = new DprFile(fileData.name, dprUri, collapsibleState);

        // If there's a DPROJ file, verify it exists and associate it
        if (fileData.hasDproj && fileData.dprojAbsolutePath) {
          try {
            const dprojUri = Uri.file(fileData.dprojAbsolutePath);
            await workspace.fs.stat(dprojUri);
            dprFile.dproj = dprojUri;

            // If executable info is in config, use it; otherwise parse it fresh
            if (fileData.executableAbsolutePath) {
              dprFile.executable = Uri.file(fileData.executableAbsolutePath);
            } else {
              // Parse executable from DPROJ
              const executableUri = await this.parseExecutableFromDproj(dprojUri);
              if (executableUri) {
                dprFile.executable = executableUri;
              }
            }
          } catch {
            // DPROJ file no longer exists, create a new non-collapsible DPR file
            const nonCollapsibleDprFile = new DprFile(fileData.name, dprUri, TreeItemCollapsibleState.None);
            dprFiles.push(nonCollapsibleDprFile);
            continue;
          }
        }

        dprFiles.push(dprFile);
      } catch {
        // DPR file no longer exists, skip it
        continue;
      }
    }

    return dprFiles;
  }

  private async parseExecutableFromDproj(dprojUri: Uri): Promise<Uri | null> {
    try {
      const dprojContent = await fs.readFile(dprojUri.fsPath, 'utf8');
      const parser = new DOMParser();
      const xmlDoc = parser.parseFromString(dprojContent, 'text/xml');

      // Find all PropertyGroup elements
      const propertyGroups = xmlDoc.getElementsByTagName('PropertyGroup');

      for (let i = 0; i < propertyGroups.length; i++) {
        const propertyGroup = propertyGroups[i];
        const dccElements = propertyGroup.getElementsByTagName('DCC_DependencyCheckOutputName');

        if (dccElements.length > 0) {
          const outputPath = dccElements[0].textContent;
          if (outputPath) {
            // The path might be relative to the DPROJ location
            const dprojDir = dirname(dprojUri.fsPath);
            const executablePath = join(dprojDir, outputPath);
            return Uri.file(executablePath);
          }
        }
      }

      return null;
    } catch (error) {
      console.error('Failed to parse DPROJ file:', error);
      return null;
    }
  }

  refresh(): void {
    this._onDidChangeTreeData.fire();
  }

  getTreeItem(element: DprTreeItem): TreeItem {
    return element;
  }

  async getChildren(element?: DprTreeItem): Promise<DprTreeItem[]> {
    if (!element) {
      // Root level - try to load from config first, then fall back to file system search
      let dprFiles: DprFile[] | null = await this.loadDprFilesFromConfig();

      if (!dprFiles || dprFiles.length === 0) {
        // Config doesn't exist or is empty, do file system search
        dprFiles = await this.getAllDprFiles();

        // Save the current list to config file (async, don't wait)
        this.saveDprListToConfig().catch(error => {
          console.error('Failed to save DPR list:', error);
        });
      }

      // Sort files alphabetically
      dprFiles.sort((a: DprFile, b: DprFile) => a.label.localeCompare(b.label));

      return dprFiles;
    } else if (element instanceof DprFile) {
      // DPR file - return DPROJ and executable as children (flat structure)
      const children: DprTreeItem[] = [];

      if (element.dproj) {
        const dprojFileName = basename(element.dproj.fsPath);
        children.push(new DprojFile(dprojFileName, element.dproj));
      }

      if (element.executable) {
        const executableFileName = basename(element.executable.fsPath);
        children.push(new ExecutableFile(executableFileName, element.executable));
      }

      return children;
    }

    return [];
  }
}

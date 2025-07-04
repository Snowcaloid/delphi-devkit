import { TreeItem, TreeDataProvider, TreeItemCollapsibleState, EventEmitter, Event, Uri, workspace, ConfigurationChangeEvent } from 'vscode';
import { basename } from 'path';
import { DelphiProjectTreeItem } from './treeItems/DelphiProjectTreeItem';
import { DelphiProject } from './treeItems/DelphiProject';
import { DprFile } from './treeItems/DprFile';
import { DprojFile } from './treeItems/DprojFile';
import { DpkFile } from './treeItems/DpkFile';
import { ExecutableFile } from './treeItems/ExecutableFile';
import { IniFile } from './treeItems/IniFile';
import { ProjectCacheData } from './types';
import { ProjectCacheManager } from './data/cacheManager';
import { ProjectDiscovery } from './data/projectDiscovery';
import { ProjectLoader } from './data/projectLoader';

/**
 * Provides a tree view of Delphi projects found in the workspace.
 *
 * Currently supports discovery of individual projects based on configuration.
 * Future versions will support .groupproj files for project grouping.
 *
 * Configuration:
 * - `delphi-utils.delphiProjects.projectPaths`: Array of glob patterns specifying where to search for projects (default: ["**"])
 * - `delphi-utils.delphiProjects.excludePatterns`: Array of glob patterns specifying paths to exclude from search
 *
 * Example settings.json:
 * ```
 * {
 *   "delphi-utils.delphiProjects.projectPaths": ["src/**", "projects/**"],
 *   "delphi-utils.delphiProjects.excludePatterns": ["&#42;&#42;/temp/&#42;&#42;", "&#42;&#42;/backup/&#42;&#42;", "&#42;&#42;/__history/&#42;&#42;"]
 * }
 * ```
 */
export class DelphiProjectsProvider implements TreeDataProvider<DelphiProjectTreeItem> {
  private _onDidChangeTreeData: EventEmitter<DelphiProjectTreeItem | undefined | null | void> = new EventEmitter<DelphiProjectTreeItem | undefined | null | void>();
  readonly onDidChangeTreeData: Event<DelphiProjectTreeItem | undefined | null | void> = this._onDidChangeTreeData.event;
  private cacheManager = new ProjectCacheManager();

  constructor() {
    // Watch for file system changes to refresh the tree (case-insensitive patterns)
    const dprWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Rr]');
    const dpkWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Kk]');
    const dprojWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Rr][Oo][Jj]');
    const iniWatcher = workspace.createFileSystemWatcher('**/*.[Ii][Nn][Ii]');
    // Future: const groupProjWatcher = workspace.createFileSystemWatcher('**/*.[Gg][Rr][Oo][Uu][Pp][Pp][Rr][Oo][Jj]');

    [dprWatcher, dpkWatcher, dprojWatcher, iniWatcher].forEach(watcher => {
      watcher.onDidCreate(() => {
        this.refresh();
        this.saveProjectsToConfig();
      });
      watcher.onDidDelete(() => {
        this.refresh();
        this.saveProjectsToConfig();
      });
      watcher.onDidChange(() => this.refresh());
    });

    // Watch for configuration changes
    workspace.onDidChangeConfiguration((event: ConfigurationChangeEvent) => {
      if (event.affectsConfiguration('delphi-utils.delphiProjects.excludePatterns') ||
          event.affectsConfiguration('delphi-utils.delphiProjects.projectPaths')) {
        this.refresh();
        this.saveProjectsToConfig();
      }
    });
  }

  private async saveProjectsToConfig(): Promise<void> {
    try {
      const projects = await ProjectDiscovery.getAllProjects();
      const configData: ProjectCacheData = {
        lastUpdated: new Date().toISOString(),
        version: '1.0',
        defaultProjects: projects.map(project => ({
          name: project.label,
          type: project.projectType,
          hasDproj: !!project.dproj,
          dprojPath: project.dproj ? workspace.asRelativePath(project.dproj) : undefined,
          dprojAbsolutePath: project.dproj?.fsPath,
          hasDpr: !!project.dpr,
          dprPath: project.dpr ? workspace.asRelativePath(project.dpr) : undefined,
          dprAbsolutePath: project.dpr?.fsPath,
          hasDpk: !!project.dpk,
          dpkPath: project.dpk ? workspace.asRelativePath(project.dpk) : undefined,
          dpkAbsolutePath: project.dpk?.fsPath,
          hasExecutable: !!project.executable,
          executablePath: project.executable ? workspace.asRelativePath(project.executable) : undefined,
          executableAbsolutePath: project.executable?.fsPath,
          hasIni: !!project.ini,
          iniPath: project.ini ? workspace.asRelativePath(project.ini) : undefined,
          iniAbsolutePath: project.ini?.fsPath
        })),
        // Future: groupProjects will be added here for .groupproj file support
        groupProjects: []
      };

      await this.cacheManager.saveCacheData(configData);
    } catch (error) {
      console.error('Failed to save Delphi projects to config:', error);
    }
  }

  /**
   * Gets the current cache structure - useful for debugging and future groupproj development.
   * @returns The current cache data or null if no cache exists
   */
  async getCurrentCacheStructure(): Promise<ProjectCacheData | null> {
    return await this.cacheManager.loadCacheData();
  }

  refresh(): void {
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: DelphiProjectTreeItem): TreeItem {
    return element;
  }

  async getChildren(element?: DelphiProjectTreeItem): Promise<DelphiProjectTreeItem[]> {
    try {
      if (!element) {
        console.log('DelphiProjectsProvider: Loading root projects...');

        // Root level - try to load from config first, then fall back to file system search
        const configData = await this.cacheManager.loadCacheData();
        let projects: DelphiProject[] | null = await ProjectLoader.loadProjectsFromConfig(configData);

        if (!projects || projects.length === 0) {
          console.log('DelphiProjectsProvider: No cached projects found, searching file system...');

          // Config doesn't exist or is empty, do file system search with timeout
          try {
            projects = await Promise.race([
              ProjectDiscovery.getAllProjects(),
              new Promise<DelphiProject[]>((_, reject) =>
                setTimeout(() => reject(new Error('Project search timed out after 30 seconds')), 30000)
              )
            ]);
          } catch (error) {
            console.error('DelphiProjectsProvider: Project search failed or timed out:', error);
            projects = [];
          }

          console.log(`DelphiProjectsProvider: Found ${projects.length} projects`);

          // Save the current list to config file (async, don't wait)
          this.saveProjectsToConfig().catch((error: any) => {
            console.error('Failed to save Delphi projects:', error);
          });
        } else {
          console.log(`DelphiProjectsProvider: Loaded ${projects.length} projects from cache`);
        }

        // Sort projects alphabetically
        projects.sort((a: DelphiProject, b: DelphiProject) => a.label.localeCompare(b.label));

        return projects;
      } else if (element instanceof DelphiProject) {
        // Delphi project - return constituent files as children
        const children: DelphiProjectTreeItem[] = [];

        if (element.dproj) {
          const dprojFileName = basename(element.dproj.fsPath);
          children.push(new DprojFile(dprojFileName, element.dproj));
        }

        if (element.dpr) {
          const dprFileName = basename(element.dpr.fsPath);
          children.push(new DprFile(dprFileName, element.dpr));
        }

        if (element.dpk) {
          const dpkFileName = basename(element.dpk.fsPath);
          children.push(new DpkFile(dpkFileName, element.dpk));
        }

        if (element.executable) {
          const executableFileName = basename(element.executable.fsPath);
          const executableItem = new ExecutableFile(
            executableFileName,
            element.executable,
            element.ini ? TreeItemCollapsibleState.Collapsed : TreeItemCollapsibleState.None
          );
          executableItem.ini = element.ini;
          children.push(executableItem);
        }

        return children;
      } else if (element instanceof ExecutableFile && element.ini) {
        // Executable file with INI - return INI as child
        const children: DelphiProjectTreeItem[] = [];
        const iniFileName = basename(element.ini.fsPath);
        children.push(new IniFile(iniFileName, element.ini));
        return children;
      }

      return [];
    } catch (error) {
      console.error('DelphiProjectsProvider: Error in getChildren:', error);
      return [];
    }
  }
}

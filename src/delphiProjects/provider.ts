import { TreeItem, TreeDataProvider, TreeItemCollapsibleState, EventEmitter, Event, Uri, workspace, ConfigurationChangeEvent, window, ProgressLocation, TreeDragAndDropController, DataTransfer, DataTransferItem } from 'vscode';
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
import { minimatch } from 'minimatch';
import { DelphiProjectsDragAndDropController } from './treeItems/DragAndDropController';

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
  private forceRefreshCache = false;
  public readonly dragAndDropController: DelphiProjectsDragAndDropController;
  private customOrder: string[] | undefined;

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
        // Removed: this.saveProjectsToConfig();
      });
      watcher.onDidDelete(() => {
        this.refresh();
        // Removed: this.saveProjectsToConfig();
      });
      watcher.onDidChange(() => this.refresh());
    });

    // Watch for configuration changes
    workspace.onDidChangeConfiguration((event: ConfigurationChangeEvent) => {
      if (event.affectsConfiguration('delphi-utils.delphiProjects.excludePatterns') ||
          event.affectsConfiguration('delphi-utils.delphiProjects.projectPaths')) {
        this.refresh();
        // Removed: this.saveProjectsToConfig();
      }
    });
    this.dragAndDropController = new DelphiProjectsDragAndDropController(() => this.refresh());
  }

  private getProjectKey(item: DelphiProjectTreeItem): string {
    // Use absolute path as unique key
    // @ts-ignore
    return item.dpr?.fsPath || item.dproj?.fsPath || item.dpk?.fsPath || item.executable?.fsPath || item.ini?.fsPath || item.resourceUri?.fsPath || item.label;
  }

  private async getCurrentOrder(): Promise<string[]> {
    if (this.customOrder) { return this.customOrder; }
    const configData = await this.cacheManager.loadCacheData();
    if (configData?.customOrder) { return configData.customOrder; }
    // Fallback: use current project order
    const projects = await ProjectDiscovery.getAllProjects();
    return projects.map(p => p.dpr?.fsPath || p.dproj?.fsPath || p.dpk?.fsPath || p.executable?.fsPath || p.ini?.fsPath || p.label);
  }

  private async saveCustomOrder(order: string[]): Promise<void> {
    const configData = await this.cacheManager.loadCacheData() || { lastUpdated: new Date().toISOString(), version: '1.0', defaultProjects: [] };
    configData.customOrder = order;
    await this.cacheManager.saveCacheData(configData);
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
        }))
        // groupProjects removed
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

  refresh(forceCache?: boolean): void {
    if (forceCache) {
      this.forceRefreshCache = true;
    }
    this._onDidChangeTreeData.fire(undefined);
  }

  getTreeItem(element: DelphiProjectTreeItem): TreeItem {
    return element;
  }

  async getChildren(element?: DelphiProjectTreeItem): Promise<DelphiProjectTreeItem[]> {
    try {
      if (!element) {
        console.log('DelphiProjectsProvider: Loading root projects...');
        let projects: DelphiProject[] | null = null;
        let configData: ProjectCacheData | null = null;
        configData = await this.cacheManager.loadCacheData();
        console.log('DelphiProjectsProvider: Loaded cache data:', configData);
        // If a group project is loaded, show only its projects
        if (configData && configData.currentGroupProject) {
          await import('vscode').then(vscode => vscode.commands.executeCommand('setContext', 'delphiUtils:groupProjectLoaded', true));
          try {
            projects = await ProjectLoader.loadProjectsFromConfig({ defaultProjects: configData.currentGroupProject.projects });
            console.log('DelphiProjectsProvider: Loaded group project projects:', projects);
          } catch (err) {
            console.error('DelphiProjectsProvider: Error loading group project projects:', err);
          }
        } else {
          await import('vscode').then(vscode => vscode.commands.executeCommand('setContext', 'delphiUtils:groupProjectLoaded', false));
          try {
            projects = await ProjectLoader.loadProjectsFromConfig(configData);
            console.log('DelphiProjectsProvider: Loaded projects from config:', projects);
            if (!projects || projects.length === 0) {
              console.log('DelphiProjectsProvider: No cached projects found, searching file system...');
              try {
                projects = await ProjectDiscovery.getAllProjects();
                console.log('DelphiProjectsProvider: Discovered projects from file system:', projects);
                await this.saveProjectsToConfig();
              } catch (error) {
                console.error('DelphiProjectsProvider: Project search failed or timed out:', error);
                window.showWarningMessage('Delphi project search failed or timed out. Please check your workspace and configuration.');
                projects = [];
              }
            }
          } catch (err) {
            console.error('DelphiProjectsProvider: Error loading projects from config:', err);
          }
        }
        if (!projects) {
          console.warn('DelphiProjectsProvider: Projects is null or undefined after all loading attempts.');
          return [];
        }
        // Custom order logic
        if (
          (configData?.customOrder && !configData.currentGroupProject) ||
          (configData?.currentGroupProject && this.dragAndDropController.groupCustomOrder)
        ) {
          // Map custom order to projects
          const keyMap = new Map<string, DelphiProject>();
          for (const p of projects) {
            const key = p.dpr?.fsPath || p.dproj?.fsPath || p.dpk?.fsPath || p.executable?.fsPath || p.ini?.fsPath || p.label;
            keyMap.set(key, p);
          }
          const customOrder = configData?.currentGroupProject && this.dragAndDropController.groupCustomOrder
            ? this.dragAndDropController.groupCustomOrder
            : configData?.customOrder!;
          const ordered = customOrder.map(key => keyMap.get(key)).filter(Boolean) as DelphiProject[];
          // Add any new projects not in customOrder at the end
          const missing = projects.filter(p => !customOrder.includes(p.dpr?.fsPath || p.dproj?.fsPath || p.dpk?.fsPath || p.executable?.fsPath || p.ini?.fsPath || p.label));
          return [...ordered, ...missing];
        }
        // Only sort if setting is enabled and not a group project
        const isGroupProject = !!(configData && configData.currentGroupProject);
        if (!isGroupProject) {
          const sortProjects = workspace.getConfiguration('delphi-utils').get<boolean>('delphiProjects.sortProjects', false);
          // Level 1: always sort by projectPaths glob order
          const config = workspace.getConfiguration('delphi-utils.delphiProjects');
          const projectPaths: string[] = config.get('projectPaths', ['**']);
          let orderedProjects: DelphiProject[] = [];
          const used = new Set<DelphiProject>();
          for (const glob of projectPaths) {
            // Find projects whose dpr/dproj/dpk path matches this glob using minimatch
            let group = projects.filter(p => {
              const absPath = p.dpr?.fsPath || p.dproj?.fsPath || p.dpk?.fsPath || '';
              const relPath = absPath ? workspace.asRelativePath(absPath).replace(/\\/g, '/') : '';
              // Use minimatch for proper glob matching
              return minimatch(relPath, glob.replace(/\\/g, '/'));
            }).filter(p => !used.has(p));
            // Level 2: sort within group if enabled
            if (sortProjects) {
              group = group.slice().sort((a, b) => a.label.localeCompare(b.label));
            }
            group.forEach(p => used.add(p));
            orderedProjects = orderedProjects.concat(group);
          }
          return orderedProjects;
        }
        return projects;
      } else if (element instanceof DelphiProject) {
        // Delphi project - return constituent files as children
        const children: DelphiProjectTreeItem[] = [];

        if (element.dproj) {
          const dprojFileName = basename(element.dproj.fsPath);
          const dprojItem = new DprojFile(dprojFileName, element.dproj);
          dprojItem.parent = element;
          children.push(dprojItem);
        }

        if (element.dpr) {
          const dprFileName = basename(element.dpr.fsPath);
          const dprItem = new DprFile(dprFileName, element.dpr);
          dprItem.parent = element;
          children.push(dprItem);
        }

        if (element.dpk) {
          const dpkFileName = basename(element.dpk.fsPath);
          const dpkItem = new DpkFile(dpkFileName, element.dpk);
          dpkItem.parent = element;
          children.push(dpkItem);
        }

        if (element.executable) {
          const executableFileName = basename(element.executable.fsPath);
          const executableItem = new ExecutableFile(
            executableFileName,
            element.executable,
            element.ini ? TreeItemCollapsibleState.Collapsed : TreeItemCollapsibleState.None
          );
          executableItem.ini = element.ini;
          executableItem.parent = element;
          children.push(executableItem);
        }

        return children;
      } else if (element instanceof ExecutableFile && element.ini) {
        // Executable file with INI - return INI as child
        const children: DelphiProjectTreeItem[] = [];
        const iniFileName = basename(element.ini.fsPath);
        const iniItem = new IniFile(iniFileName, element.ini);
        iniItem.parent = element;
        children.push(iniItem);
        return children;
      }

      return [];
    } catch (error) {
      console.error('DelphiProjectsProvider: Error in getChildren:', error);
      return [];
    }
  }
}

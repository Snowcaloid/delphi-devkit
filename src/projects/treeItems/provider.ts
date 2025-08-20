import {
  TreeItem,
  EventEmitter,
  Event,
  workspace,
  ConfigurationChangeEvent,
  commands,
  window,
  TreeDataProvider,
  Uri
} from "vscode";
import { DelphiProjectTreeItem } from "./delphiProjectTreeItem";
import { DelphiProjectTreeItemType, WorkspaceViewMode } from "../types";
import { DelphiProject } from "./delphiProject";
import { ProjectDiscovery } from "../data/projectDiscovery";
import { DelphiProjectsDragAndDropController } from "./DragAndDropController";
import { GroupProjectEntity, ProjectEntity } from "../../db/entities";
import { GroupProjectPicker } from "../pickers/groupProjPicker";
import { Runtime, RuntimeProperty } from "../../runtime";
import { basename } from "path";
import { AppDataSource } from "../../db/datasource";
import { Projects } from "../../constants";
import { SelectedItemDecorator } from "./selectedItemDecorator";

type NullableTreeItem = DelphiProjectTreeItem | undefined | null | void;

export class DelphiProjectsProvider
  implements TreeDataProvider<DelphiProjectTreeItem>
{
  private _onDidChangeTreeData: EventEmitter<NullableTreeItem> =
    new EventEmitter<NullableTreeItem>();
  private clearCache: boolean = false;
  public readonly onDidChangeTreeData: Event<NullableTreeItem> =
    this._onDidChangeTreeData.event;
  public readonly dragAndDropController: DelphiProjectsDragAndDropController;
  public readonly selectedItemDecorator: SelectedItemDecorator;

  private createWatchers(): void {
    if (
      !workspace
        .getConfiguration(Projects.Config.Key)
        .get<boolean>(Projects.Config.Discovery.UseFileSystemWatchers, false)
    ) {
      return;
    }
    const dprojWatcher = workspace.createFileSystemWatcher("**/*.[Dd][Pp][Rr][Oo][Jj]", false, true);
    const dprWatcher = workspace.createFileSystemWatcher("**/*.[Dd][Pp][Rr]", false, true);
    const dpkWatcher = workspace.createFileSystemWatcher("**/*.[Dd][Pp][Kk]", false, true);
    const iniWatcher = workspace.createFileSystemWatcher("**/*.[Ii][Nn][Ii]", false, true);
    const exeWatcher = workspace.createFileSystemWatcher("**/*.[Ee][Xx][Ee]", false, true);
    const watchers = [dprojWatcher, dprWatcher, dpkWatcher, iniWatcher, exeWatcher];

    watchers.forEach((watcher) => {
      watcher.onDidCreate(() => {
        this.refreshTreeView();
      });
      watcher.onDidDelete(() => {
        this.refreshTreeView();
      });
    });
    Runtime.extension.subscriptions.push(...watchers);
    const gitCheckoutDelay = workspace.getConfiguration(Projects.Config.Key).get<number>(Projects.Config.GitCheckoutDelay, 30000);
    let updateRequest: NodeJS.Timeout | undefined = undefined;
    Runtime.subscribe(
      (prop, val: any) => {
        switch (prop) {
          case RuntimeProperty.WorkspaceAvailable:
            watchers.forEach((watcher) => watcher.dispose());
            clearTimeout(updateRequest);
            if (val) {
              this.createWatchers();
            }
            break;
          case RuntimeProperty.Workspace:
            watchers.forEach((watcher) => watcher.dispose());
            clearTimeout(updateRequest);
            updateRequest = setTimeout(() => {
              this.createWatchers();
            }, gitCheckoutDelay);
            break;
        }
      }
    );
  }

  private createConfigurationWatcher() {
    workspace.onDidChangeConfiguration((event: ConfigurationChangeEvent) => {
      if (
        event.affectsConfiguration(Projects.Config.full(Projects.Config.Discovery.ProjectPaths)) ||
        event.affectsConfiguration(Projects.Config.full(Projects.Config.Discovery.ExcludePatterns))
      ) {
        this.refreshTreeView();
      }
    });
  }

  private createCommands() {
    const refreshDelphiProjects = commands.registerCommand(Projects.Command.Refresh, async () => {
      if (!await Runtime.assertWorkspaceAvailable()) {
        window.showWarningMessage('No workspace available. Please open a workspace to refresh Delphi projects.');
        return;
      }
      await this.refreshTreeView(true);
    });

    const pickGroupProjectCommand = commands.registerCommand(Projects.Command.PickGroupProject, async () => {
      const uri = await this.groupProjPicker.pickGroupProject();
      if (!uri) { return; }
      let needToFindProjects = false;

      let ws = await Runtime.db.modify(async (ws) => {
        let groupProj = await Runtime.db.getGroupProject(uri);
        if (groupProj) {
          ws.currentGroupProject = groupProj;
          return ws;
        }
        groupProj = new GroupProjectEntity();
        groupProj.name = basename(uri.fsPath);
        groupProj.path = uri.fsPath;
        needToFindProjects = true;
        ws.currentGroupProject = groupProj;
        return ws;
      });
      if (needToFindProjects) {
        ws = await Runtime.db.modify(async (ws) => {
          ws.currentGroupProject!.projects = await new ProjectDiscovery().findFilesFromGroupProj(ws.currentGroupProject!);
          return ws;
        });
      }
      await this.refreshTreeView();
      window.showInformationMessage(`Loaded group project: ${ws?.currentGroupProject?.name}`);
    });

    const unloadGroupProjectCommand = commands.registerCommand(Projects.Command.UnloadGroupProject, async () => {
      await Runtime.db.modify(async (ws) => {
        if (ws.viewMode === WorkspaceViewMode.GroupProject) {
          ws.currentGroupProject = null;
          ws.lastUpdated = new Date();
        }
        return ws;
      });
      await this.refreshTreeView();
      window.showInformationMessage('Unloaded group project. Showing default projects (if discovery is enabled).');
    });

    const editDefaultIniCommand = commands.registerCommand(Projects.Command.EditDefaultIni, async () => {
      const defaultIniPath = Runtime.extension.asAbsolutePath("dist/default.ini");
      try {
        await commands.executeCommand("vscode.open", Uri.file(defaultIniPath));
      } catch (error) {
        window.showErrorMessage(`Failed to open default.ini: ${error}`);
      }
    });

    Runtime.extension.subscriptions.push(...[
      refreshDelphiProjects,
      pickGroupProjectCommand,
      unloadGroupProjectCommand,
      editDefaultIniCommand
    ]);
  }

  constructor(
    private readonly groupProjPicker: GroupProjectPicker = new GroupProjectPicker()
  ) {
    this.createWatchers();
    this.createConfigurationWatcher();
    this.createCommands();
    this.dragAndDropController = new DelphiProjectsDragAndDropController();
    this.selectedItemDecorator = new SelectedItemDecorator();
    Runtime.extension.subscriptions.push(
      window.registerFileDecorationProvider(this.selectedItemDecorator)
    );
    Runtime.subscribe((property, newValue, oldValue) => {
      switch (property) {
        case RuntimeProperty.Workspace:
          this.refreshTreeView();
          break;
      }
    });
  }

  private async marshalAll(items?: DelphiProjectTreeItem[]): Promise<ProjectEntity[]> {
    const projects = items || await this.getChildren();
    return Promise.all(
      projects
        .filter((project) => project instanceof DelphiProject)
        .map((project) => project.marshal())
    );
  }

  public async save(items?: DelphiProjectTreeItem[]): Promise<void> {
    await Runtime.db.modify(async (ws) => {
      ws.lastUpdated = new Date();
      switch (ws.viewMode) {
        case WorkspaceViewMode.GroupProject: {
          if (ws.currentGroupProject) {
            ws.currentGroupProject.projects = await this.marshalAll(items);
          }
          break;
        }
        case WorkspaceViewMode.Discovery: {
          ws.discoveredProjects = await this.marshalAll(items);
          break;
        }
      }
      return ws;
    });
  }

  getTreeItem(element: DelphiProjectTreeItem): TreeItem {
    return element;
  }

  private createChildrenForProject(
    project: DelphiProject
  ): DelphiProjectTreeItem[] {
    const children: DelphiProjectTreeItem[] = [];
    project.createChild(DelphiProjectTreeItemType.DprojFile, children);
    project.createChild(DelphiProjectTreeItemType.DprFile, children);
    project.createChild(DelphiProjectTreeItemType.DpkFile, children);
    project.createChild(DelphiProjectTreeItemType.ExecutableFile, children);
    project.createChild(DelphiProjectTreeItemType.IniFile, children);
    return children;
  }

  private async createTreeItems(): Promise<DelphiProjectTreeItem[]> {
    if (this.clearCache) {
      this.clearCache = false;
      await AppDataSource.reset();
    }
    const modifiedWorkspace = await Runtime.db.modify(async (ws) => {
      commands.executeCommand( // for VS code items to be visible/invisible
        "setContext",
        Projects.Context.IsGroupProjectView,
        ws.viewMode === WorkspaceViewMode.GroupProject
      );
      commands.executeCommand(
        "setContext",
        Projects.Context.IsProjectSelected,
        false,
      );
      commands.executeCommand(
        "setContext",
        Projects.Context.DoesSelectedProjectHaveExe,
        false
      );
      await Runtime.extension.workspaceState.update(
        Projects.Variables.IsGroupProjectView,
        ws.viewMode === WorkspaceViewMode.GroupProject
      );
      switch (ws.viewMode) {
        case WorkspaceViewMode.GroupProject:
          if (ws.currentGroupProject) {
            ws.currentGroupProject.projects = await Runtime.db.removeNonExistentFiles(ws.currentGroupProject.projects);
          }
          break;
        case WorkspaceViewMode.Empty:
          const config = workspace.getConfiguration(Projects.Config.Key);
          if (config && !config.get<boolean>(Projects.Config.Discovery.Enable, true)) {
            break;
          }
        case WorkspaceViewMode.Discovery:
          ws.discoveredProjects = await Runtime.db.removeNonExistentFiles(ws.discoveredProjects);
          if (!ws.discoveredProjects || ws.discoveredProjects.length === 0) {
            ws.discoveredProjects = await new ProjectDiscovery().findAllProjects();
          }
          break;
      }
      return ws;
    });
    switch (modifiedWorkspace?.viewMode) {
      case WorkspaceViewMode.Discovery:
        return (await Promise.all(
          modifiedWorkspace.discoveredProjects.map(async (project) => {
            return DelphiProject.fromData(modifiedWorkspace, project);
          })
        )).sort((a, b) => a.sortValue.localeCompare(b.sortValue));
      case WorkspaceViewMode.GroupProject:
        if (modifiedWorkspace.currentGroupProject) {
          return (await Promise.all(
            modifiedWorkspace.currentGroupProject.projects.map(async (project) => {
              return DelphiProject.fromData(modifiedWorkspace, project);
            })
          )).sort((a, b) => a.sortValue.localeCompare(b.sortValue));
        } else {
          return [];
        }
      default:
        return [];
    }
  }

  async getChildren(
    element?: DelphiProjectTreeItem
  ): Promise<DelphiProjectTreeItem[]> {
    if (!element || this.clearCache) {
      return this.createTreeItems();
    } else if (element instanceof DelphiProject) {
      return this.createChildrenForProject(element);
    }
    return [];
  }

  public async refreshTreeView(clearCache: boolean = false): Promise<void> {
    this.clearCache = clearCache;
    this._onDidChangeTreeData.fire(undefined);
  }
}

import { TreeItem, EventEmitter, Event, workspace, ConfigurationChangeEvent, TreeDataProvider, window, Uri } from 'vscode';
import { BaseFileItem } from './items/baseFile';
import { DelphiProjectTreeItemType } from '../../types';
import { ProjectItem } from './items/project';
// import { DelphiProjectsDragAndDropController } from "./DragAndDropController";
import { Runtime } from '../../runtime';
import { PROJECTS } from '../../constants';
import { TreeItemDecorator } from './treeItemDecorator';
import { WorkspaceItem } from './items/workspaceItem';
import { GroupProjectTreeDragDropController, WorkspaceTreeDragDropController } from './dragAndDrop';

type NullableTreeItem = BaseFileItem | undefined | null | void;

export abstract class DelphiProjectsTreeView implements TreeDataProvider<TreeItem> {
  private changeEventEmitter: EventEmitter<NullableTreeItem> = new EventEmitter<NullableTreeItem>();
  public readonly onDidChangeTreeData: Event<NullableTreeItem> = this.changeEventEmitter.event;
  public projects: ProjectItem[] = [];

  private createWatchers(): void {
    const dprojWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Rr][Oo][Jj]', false, true);
    const dprWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Rr]', false, true);
    const dpkWatcher = workspace.createFileSystemWatcher('**/*.[Dd][Pp][Kk]', false, true);
    const exeWatcher = workspace.createFileSystemWatcher('**/*.[Ee][Xx][Ee]', false, true);
    const iniWatcher = workspace.createFileSystemWatcher('**/*.[Ii][Nn][Ii]', false, true);
    const watchers = [dprojWatcher, dprWatcher, dpkWatcher, iniWatcher, exeWatcher];

    watchers.forEach((watcher) => {
      watcher.onDidCreate((file: Uri) => {
        this.onWatcherEvent(file);
      });
      watcher.onDidDelete((file: Uri) => {
        this.onWatcherEvent(file);
      });
    });
    Runtime.extension.subscriptions.push(...watchers);
  }

  private isRelevantFile(file: Uri): boolean {
    for (const item of this.projects)
      if (
        item.entity.directory === file.fsPath ||
        item.projectDproj?.fsPath === file.fsPath ||
        item.projectDpr?.fsPath === file.fsPath ||
        item.projectDpk?.fsPath === file.fsPath ||
        item.projectExe?.fsPath === file.fsPath ||
        item.projectIni?.fsPath === file.fsPath
      )
        return true;

    return false;
  }

  private onWatcherEvent(file: Uri): void {
    if (this.isRelevantFile(file)) this.refresh();
  }

  private createConfigurationWatcher() {
    workspace.onDidChangeConfiguration((event: ConfigurationChangeEvent) => {
      if (
        event.affectsConfiguration(PROJECTS.CONFIG.full(PROJECTS.CONFIG.DISCOVERY.PROJECT_PATHS)) ||
        event.affectsConfiguration(PROJECTS.CONFIG.full(PROJECTS.CONFIG.DISCOVERY.EXCLUDE_PATTERNS))
      )
        this.refresh();
    });
  }

  constructor() {
    this.createWatchers();
    this.createConfigurationWatcher();
  }

  getTreeItem(element: TreeItem): TreeItem {
    return element;
  }

  private createChildrenForProject(project: ProjectItem): BaseFileItem[] {
    const children: BaseFileItem[] = [];
    project.createChild(DelphiProjectTreeItemType.DprojFile, children);
    project.createChild(DelphiProjectTreeItemType.DprFile, children);
    project.createChild(DelphiProjectTreeItemType.DpkFile, children);
    project.createChild(DelphiProjectTreeItemType.ExecutableFile, children);
    project.createChild(DelphiProjectTreeItemType.IniFile, children);
    return children;
  }

  protected abstract loadTreeItemsFromDatabase(): Promise<TreeItem[]>;

  protected get itemsLoaded(): boolean {
    return !!this.projects && this.projects.length > 0;
  }

  protected get loadedItems(): TreeItem[] {
    return this.projects;
  }

  async getChildren(element?: TreeItem): Promise<TreeItem[]> {
    if (!element)
      if (this.itemsLoaded) return this.loadedItems;
      else return await this.loadTreeItemsFromDatabase();
    else if (element instanceof ProjectItem) return this.createChildrenForProject(element);
    else if (element instanceof WorkspaceItem) return element.projects;

    return [];
  }

  public async refresh(): Promise<void> {
    this.projects = [];
    this.changeEventEmitter.fire(undefined);
  }
}

export class WorkspacesTreeView extends DelphiProjectsTreeView {
  public workspaceItems: WorkspaceItem[] = [];

  constructor(
    public readonly dragAndDropController = new WorkspaceTreeDragDropController(),
    public readonly treeItemDecorator = new TreeItemDecorator()
  ) {
    super();
    Runtime.extension.subscriptions.push(
      window.createTreeView(PROJECTS.VIEW.WORKSPACES, {
        treeDataProvider: this,
        dragAndDropController: this.dragAndDropController,
        showCollapseAll: true
      }),
      window.registerFileDecorationProvider(this.treeItemDecorator)
    );
  }

  protected get itemsLoaded(): boolean {
    return !!this.workspaceItems && this.workspaceItems.length > 0;
  }

  protected get loadedItems(): TreeItem[] {
    return this.workspaceItems;
  }

  protected async loadTreeItemsFromDatabase(): Promise<TreeItem[]> {
    let data = Runtime.projectsData;
    Runtime.setContext(PROJECTS.CONTEXT.IS_PROJECT_SELECTED, !!data?.active_project_id);
    Runtime.setContext(PROJECTS.CONTEXT.DOES_SELECTED_PROJECT_HAVE_EXE, !!Runtime.activeProject?.exe);
    this.workspaceItems = data?.workspaces.map((ws) => new WorkspaceItem(ws)) || [];
    this.workspaceItems = this.workspaceItems.sort((a, b) => a.workspace.sort_rank.localeCompare(b.workspace.sort_rank));
    this.projects = this.workspaceItems.flatMap((ws) => ws.projects);
    return this.workspaceItems;
  }

  public async refresh(): Promise<void> {
    this.workspaceItems = [];
    await super.refresh();
  }

  public getWorkspaceItemByTreeItem(item: TreeItem): WorkspaceItem | undefined {
    if (item instanceof WorkspaceItem) return item;
    else if (item instanceof BaseFileItem)
      return this.workspaceItems.find((wsItem) => wsItem.projects.some((projItem) => projItem.link.id === item.project.link.id));
  }
}

export class GroupProjectTreeView extends DelphiProjectsTreeView {
  constructor(public readonly dragAndDropController = new GroupProjectTreeDragDropController()) {
    super();
    Runtime.extension.subscriptions.push(
      window.createTreeView(PROJECTS.VIEW.GROUP_PROJECT, {
        treeDataProvider: this,
        dragAndDropController: this.dragAndDropController,
        showCollapseAll: true
      })
    );
  }

  protected async loadTreeItemsFromDatabase(): Promise<TreeItem[]> {
    const groupProject = Runtime.projectsData?.group_project;
    Runtime.setContext(PROJECTS.CONTEXT.IS_GROUP_PROJECT_OPENED, !!groupProject);
    if (groupProject)
      for (const link of groupProject.project_links)
        if (Runtime.getProjectOfLink(link)) {
          const item = ProjectItem.fromData(link);
          this.projects.push(item);
        }

    return this.projects;
  }
}

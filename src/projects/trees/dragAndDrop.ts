import { TreeDragAndDropController, DataTransfer, DataTransferItem, TreeItem, window } from 'vscode';
import { Runtime } from '../../runtime';
import { LexoSorter } from '../../utils/lexoSorter';
import { ProjectItem } from './items/project';
import { Entities } from '../../db/entities';
import { assertError } from '../../utils';
import { WorkspaceItem } from './items/workspaceItem';
import { PROJECTS } from '../../constants';
import { BaseFileItem, MainProjectItem } from './items/baseFile';

interface Target {
  isEmpty: boolean;
  isProject: boolean;
  isWorkspace: boolean;
  entity: {
    project?: Entities.Project;
    workspace?: Entities.Workspace;
    workspaceLink?: Entities.WorkspaceLink;
  };
  item: {
    project?: MainProjectItem;
    workspace?: WorkspaceItem;
  };
}

interface Source extends Target {
  isDraggedFromGroupProject: boolean;
  isFSFileList?: boolean;
  fileList?: string[];
}

class ExtendedTransferInfo {
  public readonly source: Source;
  public readonly target: Target;
  constructor(source: TreeItem, target: TreeItem | undefined) {
    const evaluatedSource = this.evaluate(source);
    this.source = {
      isDraggedFromGroupProject: evaluatedSource.isProject && !evaluatedSource.entity.workspace,
      ...this.evaluate(source)
    };
    this.target = this.evaluate(target);
  }

  public validate(): boolean {
    if (!this.target.entity.workspace || !this.target.item.workspace) return false;
    if (this.source.isProject) {
      if (this.target.isEmpty) return false;
      if (!this.source.entity.project || !this.source.item.project) return false;
      if (!this.source.isDraggedFromGroupProject)
        // we are dragging from a workspace - all workspace info must exist
        if (!this.source.entity.workspaceLink || !this.source.entity.workspace || !this.source.item.workspace)
          return false;
    }
    if (this.target.isProject && (!this.target.entity.project || !this.target.item.project || !this.target.entity.workspaceLink)) return false;

    if (this.source.isWorkspace) {
      if (!this.source.entity.workspace || !this.source.item.workspace) return false;
      if (!this.target.entity.workspace || !this.target.item.workspace) return false;
    }
    if (this.target.isWorkspace) {
      if (!this.target.entity.workspace || !this.target.item.workspace) return false;
      if (!this.source.isDraggedFromGroupProject && (this.source.entity.workspace!.id === this.target.entity.workspace!.id)) return false;
    }
    return true;
  }

  private evaluate(target: TreeItem | undefined): Target {
    if (!target)
      return {
        isEmpty: true,
        isProject: false,
        isWorkspace: false,
        entity: {},
        item: {}
      };

    const isProject = target instanceof BaseFileItem;
    const isWorkspace = target instanceof WorkspaceItem;
    let projectItem: MainProjectItem | undefined = undefined;
    let workspaceItem: WorkspaceItem | undefined = undefined;
    let projectEntity: Entities.Project | undefined = undefined;
    let workspaceEntity: Entities.Workspace | undefined = undefined;
    let linkEntity: Entities.WorkspaceLink | undefined = undefined;
    if (isProject) {
      projectItem = target.project;
      projectEntity = target.project.entity;
      if (Runtime.projects.workspacesTreeView.projects.find((item) => item.link.id === target.project.link.id)) {
        workspaceItem = Runtime.projects.workspacesTreeView.getWorkspaceByTreeItem(target);
        workspaceEntity = target.project.link.workspaceSafe || undefined;
        linkEntity = target.project.link as Entities.WorkspaceLink;
      }
    }
    if (isWorkspace) {
      workspaceItem = target;
      workspaceEntity = target.workspace;
    }
    return {
      isEmpty: false,
      isProject: isProject,
      isWorkspace: isWorkspace,
      entity: {
        project: projectEntity,
        workspace: workspaceEntity,
        workspaceLink: linkEntity
      },
      item: {
        project: projectItem,
        workspace: workspaceItem
      }
    };
  }
}

export class WorkspaceTreeDragDropController implements TreeDragAndDropController<TreeItem> {
  public readonly dragMimeTypes = [PROJECTS.MIME_TYPES.WORKSPACE, PROJECTS.MIME_TYPES.WORKSPACE_PROJECT];
  public readonly dropMimeTypes = [
    PROJECTS.MIME_TYPES.WORKSPACE,
    PROJECTS.MIME_TYPES.WORKSPACE_PROJECT,
    PROJECTS.MIME_TYPES.GROUP_PROJECT_CHILD,
    PROJECTS.MIME_TYPES.FS_FILES
  ];

  public async handleDrag(source: TreeItem[], dataTransfer: DataTransfer): Promise<void> {
    if (!source || source.length !== 1) return;
    const item = source[0];
    if (
      !assertError(
        item instanceof ProjectItem || item instanceof WorkspaceItem,
        'Invalid item type. Drag and drop is only supported for Delphi projects and workspace items.'
      )
    )
      return;

    if (item instanceof ProjectItem) dataTransfer.set(PROJECTS.MIME_TYPES.WORKSPACE_PROJECT, new DataTransferItem(item.link.id));
    else if (item instanceof WorkspaceItem) dataTransfer.set(PROJECTS.MIME_TYPES.WORKSPACE, new DataTransferItem(item.workspace.id));
  }

  public async handleDrop(target: TreeItem | undefined, dataTransfer: DataTransfer): Promise<void> {
    let hasFiles = false;
    for (const [mime, item] of dataTransfer) {
      if (!item?.value) continue;
      switch(mime) {
        case PROJECTS.MIME_TYPES.WORKSPACE_PROJECT:
          return await this.handleDropProject(target, item);
        case PROJECTS.MIME_TYPES.WORKSPACE:
          return await this.handleDropWorkspace(target, item);
        case PROJECTS.MIME_TYPES.GROUP_PROJECT_CHILD:
          return await this.handleDropProject(target, item, {
            sourceIsGroupProject: true
          });
        case PROJECTS.MIME_TYPES.FS_FILES: 
          hasFiles = true;
      }
    } 
    // if we reach this, it means we haven't handled any other type
    if (hasFiles) await window.showInformationMessage('Drag-Drop of files from file system is coming soon.');
  }

  private async handleDropProject(
    target: TreeItem | undefined,
    transferItem: DataTransferItem,
    options?: { sourceIsGroupProject: boolean }
  ): Promise<void> {
    const id =
      typeof transferItem.value === 'number' ? transferItem.value : typeof transferItem.value === 'string' ? parseInt(transferItem.value) : NaN;
    if (isNaN(id)) return;
    let source: ProjectItem | undefined;
    if (options?.sourceIsGroupProject) source = Runtime.projects.groupProjectTreeView.projects.find((proj) => proj.link.id === id);
    else source = Runtime.projects.workspacesTreeView.projects.find((proj) => proj.link.id === id);
    if (!source) return;
    const transfer = new ExtendedTransferInfo(source, target);
    if (transfer.source.isProject) await this.dropProject(transfer);
    await Runtime.projects.workspacesTreeView.refresh();
  }

  private async handleDropWorkspace(target: TreeItem | undefined, transferItem: DataTransferItem): Promise<void> {
    const id =
      typeof transferItem.value === 'number' ? transferItem.value : typeof transferItem.value === 'string' ? parseInt(transferItem.value) : NaN;
    if (isNaN(id)) return;
    const source = Runtime.projects.workspacesTreeView.workspaceItems.find((ws) => ws.workspace.id === id);
    if (!source) return;
    const transfer = new ExtendedTransferInfo(source, target);
    if (transfer.source.isWorkspace) await this.dropWorkspace(transfer);
    await Runtime.projects.workspacesTreeView.refresh();
  }

  private async dropProject(transfer: ExtendedTransferInfo): Promise<void> {
    // validate all required combinations
    if (!transfer.validate()) return;
    const source = transfer.source;
    const target = transfer.target;
    if (source.isDraggedFromGroupProject)
      return await this.addProjectToWorkspace(source.entity.project!, target.entity.workspace!, target.entity.workspaceLink);

    if (target.isProject) {
      const isSameWorkspace = source.entity.workspace!.id === target.entity.workspace!.id;
      if (isSameWorkspace) {
        const sorter = new LexoSorter(target.entity.workspace!.projects);
        target.entity.workspace!.projects = sorter.reorder(source.entity.workspaceLink!, target.entity.workspaceLink!);
        await Runtime.db.save(target.entity.workspace!);
      } else {
        // moving to a different workspace
        source.entity.workspace!.projects = source.entity.workspace!.projects.filter((link) => link.id !== source.entity.workspaceLink!.id);
        source.entity.workspaceLink!.workspace = target.entity.workspace!;
        target.entity.workspace!.projects.push(source.entity.workspaceLink!);
        const sorter = new LexoSorter(target.entity.workspace!.projects);
        target.entity.workspace!.projects = sorter.reorder(source.entity.workspaceLink!, target.entity.workspaceLink!);
        await Runtime.db.save(source.entity.workspaceLink!);
      }
    } else if (target.isWorkspace) {
      source.entity.workspace!.projects = source.entity.workspace!.projects.filter((link) => link.id !== source.entity.workspaceLink!.id);
      source.entity.workspaceLink!.workspace = target.entity.workspace!;
      target.entity.workspace!.projects.push(source.entity.workspaceLink!);
      target.entity.workspace!.projects = new LexoSorter(target.entity.workspace!.projects).items;
      await Runtime.db.save(source.entity.workspaceLink!);
    }
  }

  private async addProjectToWorkspace(
    project: Entities.Project,
    workspace: Entities.Workspace,
    beforeLink: Entities.WorkspaceLink | undefined
  ): Promise<void> {
    const link = new Entities.WorkspaceLink();
    link.project = project;
    link.workspace = workspace;
    workspace.projects.push(link);
    const sorter = new LexoSorter(workspace.projects);
    if (beforeLink) workspace.projects = sorter.reorder(link, beforeLink);
    else workspace.projects = sorter.items;

    await Runtime.db.save(workspace);
  }

  private async dropWorkspace(transfer: ExtendedTransferInfo): Promise<void> {
    const source = transfer.source;
    const target = transfer.target;
    const config = Runtime.configEntity;
    if (target.isEmpty) {
      const length = config.workspaces?.length || 0;
      if (length < 2) return;
      // already last?
      if (config.workspaces.findIndex((ws) => ws.id === source.entity.workspace!.id) === length - 1) return;
      const lastWorkspace = config.workspaces[length - 1];
      config.workspaces = new LexoSorter(config.workspaces).reorder(source.entity.workspace!, lastWorkspace);
    } else {
      const sorter = new LexoSorter(config.workspaces || []);
      config.workspaces = sorter.reorder(source.entity.workspace!, target.entity.workspace!);
    }
    await Runtime.db.save(config);
  }
}

export class GroupProjectTreeDragDropController implements TreeDragAndDropController<TreeItem> {
  public readonly dragMimeTypes = [PROJECTS.MIME_TYPES.GROUP_PROJECT_CHILD];
  public readonly dropMimeTypes = [];

  public async handleDrag(source: BaseFileItem[], dataTransfer: DataTransfer): Promise<void> {
    if (!source || source.length !== 1) return;
    const item = source[0];
    if (!item.isProjectItem) return;
    dataTransfer.set(PROJECTS.MIME_TYPES.GROUP_PROJECT_CHILD, new DataTransferItem(item.project.link.id));
  }

  public async handleDrop(): Promise<void> {
    window.showInformationMessage('Group project items cannot be adjusted.');
  }
}

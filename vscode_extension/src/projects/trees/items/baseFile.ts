import { TreeItem, TreeItemCollapsibleState, Uri } from 'vscode';
import { DelphiProjectTreeItemType, ProjectType } from '../../../types';
import { PROJECTS } from '../../../constants';
import { Entities } from '../../../db/entities';

export interface MainProjectItem {
  entity: Entities.Project;
  link: Entities.ProjectLink;
  resourceUri: Uri;
}

export abstract class BaseFileItem extends TreeItem {
  public project: MainProjectItem;

  constructor(
    public readonly itemType: DelphiProjectTreeItemType,
    public readonly label: string,
    public resourceUri: Uri,
    project?: MainProjectItem
  ) {
    super(label, itemType === DelphiProjectTreeItemType.Project ? TreeItemCollapsibleState.Collapsed : TreeItemCollapsibleState.None);
    this.project = project || (this as unknown as MainProjectItem);
    this.contextValue = PROJECTS.CONTEXT.PROJECT_FILE;
    this.tooltip = this.resourceUri.fsPath;
  }

  public get isProjectItem(): boolean {
    return this.collapsibleState !== TreeItemCollapsibleState.None; // Only project items are collapsible
  }

  public get projectUri(): Uri {
    return this.project.resourceUri;
  }

  public get projectSortValue(): string {
    return this.project.link.sortValue;
  }

  public get projectDproj(): Uri | undefined {
    const path = this.project.entity.dproj;
    if (path) return Uri.file(path);
  }

  public get projectDpr(): Uri | undefined {
    const path = this.project.entity.dpr;
    if (path) return Uri.file(path);
  }

  public get projectDpk(): Uri | undefined {
    const path = this.project.entity.dpk;
    if (path) return Uri.file(path);
  }

  public get projectExe(): Uri | undefined {
    const path = this.project.entity.exe;
    if (path) return Uri.file(path);
  }

  public get projectIni(): Uri | undefined {
    const path = this.project.entity.ini;
    if (path) return Uri.file(path);
  }

  public get projectType(): ProjectType {
    if (this.projectDpk) return ProjectType.Package;
    return ProjectType.Application;
  }
}

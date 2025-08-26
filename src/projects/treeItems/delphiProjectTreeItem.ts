import { TreeItem, TreeItemCollapsibleState, Uri } from 'vscode';
import { DelphiProjectTreeItemType } from '../../types';
import { ProjectType } from './delphiProject';
import { ProjectEntity } from '../../db/entities';

export interface DelphiProjectMainTreeItem {
  entity: ProjectEntity;
  resourceUri: Uri;
}

export abstract class DelphiProjectTreeItem extends TreeItem {
  public project: DelphiProjectMainTreeItem;

  constructor(
    public readonly itemType: DelphiProjectTreeItemType,
    public readonly label: string,
    public readonly resourceUri: Uri,
    public readonly projectType: ProjectType
  ) {
    super(label, itemType === DelphiProjectTreeItemType.Project ? TreeItemCollapsibleState.Collapsed : TreeItemCollapsibleState.None);
    this.tooltip = this.resourceUri.fsPath;
  }

  public get projectUri(): Uri {
    return this.project.resourceUri;
  }

  public get projectSortValue(): string {
    return this.project.entity.sortValue;
  }

  public get projectDproj(): Uri | undefined {
    if (this.project.entity.dprojPath) {
      return Uri.file(this.project.entity.dprojPath);
    }
  }

  public get projectDpr(): Uri | undefined {
    if (this.project.entity.dprPath) {
      return Uri.file(this.project.entity.dprPath);
    }
  }

  public get projectDpk(): Uri | undefined {
    if (this.project.entity.dpkPath) {
      return Uri.file(this.project.entity.dpkPath);
    }
  }

  public get projectExe(): Uri | undefined {
    if (this.project.entity.exePath) {
      return Uri.file(this.project.entity.exePath);
    }
  }

  public get projectIni(): Uri | undefined {
    if (this.project.entity.iniPath) {
      return Uri.file(this.project.entity.iniPath);
    }
  }
}

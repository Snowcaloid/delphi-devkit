import { TreeItemCollapsibleState, ThemeIcon, Uri, commands } from "vscode";
import { DelphiProjectMainTreeItem, DelphiProjectTreeItem } from "./delphiProjectTreeItem";
import { DelphiProjectTreeItemType } from "../../types";
import { DprojFile } from "./dprojFile";
import { DprFile } from "./dprFile";
import { IniFile } from "./iniFile";
import { ExeFile } from "./exeFile";
import { DpkFile } from "./dpkFile";
import { basename } from "path";
import { ProjectEntity, WorkspaceEntity } from "../../db/entities";
import { Runtime } from "../../runtime";
import { SortedItem } from "../../utils/lexoSorter";
import { fileExists } from "../../utils";
import { Projects } from "../../constants";

export enum ProjectType {
  Application = "application",
  Package = "package",
}

export class DelphiProject extends DelphiProjectTreeItem implements DelphiProjectMainTreeItem, SortedItem {
  constructor(
    public entity: ProjectEntity,
    label: string,
    projectType: ProjectType,
    selected: boolean = false
  ) {
    const path = entity.dprojPath || entity.dprPath || entity.dpkPath || entity.exePath || entity.iniPath;
    if (!path) { throw new Error("At least one project file must be provided."); }
    const uriPath = path.replace(basename(path), label);
    if (selected) {
      commands.executeCommand(
        "setContext",
        Projects.Context.IsProjectSelected,
        true,
      );
      commands.executeCommand(
        "setContext",
        Projects.Context.DoesSelectedProjectHaveExe,
        !!entity.exePath
      );
    }
    const resourceUri = selected ?
      Uri.from({ scheme: Projects.Scheme.Selected, path: uriPath }) :
      Uri.from({ scheme: Projects.Scheme.Default, path: uriPath });
    super(
      DelphiProjectTreeItemType.Project,
      label,
      resourceUri,
      projectType,
    );
    this.project = this;
    this.contextValue = "delphiProject";
    this.setIcon();
  }

  public set sortValue(value: string) {
    this.entity.sortValue = value;
  }

  public get sortValue(): string {
    return this.entity.sortValue;
  }

  public static fromData(workspace: WorkspaceEntity, entity: ProjectEntity): DelphiProject {
    const selected =
      workspace.currentGroupProject?.currentProject?.id === entity.id ||
      workspace.currentProject?.id === entity.id;
    const project = new DelphiProject(
      entity,
      entity.name,
      <ProjectType>entity.type,
      selected
    );
    project.sortValue = entity.sortValue;
    project.updateCollapsibleState();
    return project;
  }

  setIcon(): void {
    if (this.projectDpk) {
      this.iconPath = new ThemeIcon("package");
    } else if (this.projectDpr) {
      this.iconPath = new ThemeIcon("run");
    } else {
      this.iconPath = new ThemeIcon("symbol-class");
    }
  }

  // Update collapsible state based on children
  updateCollapsibleState(): void {
    const hasChildren = !!(
      this.projectDproj ||
      this.projectDpr ||
      this.projectDpk ||
      this.projectExe ||
      this.projectIni
    );
    this.collapsibleState = hasChildren
      ? TreeItemCollapsibleState.Collapsed
      : TreeItemCollapsibleState.None;
  }

  async setDproj(value: Uri, save: boolean = false): Promise<void> {
    this.entity.dprojPath = value.fsPath;
    if (save) {
      await Runtime.projects.treeView.save();
    }
  }

  async setDpr(value: Uri, save: boolean = false): Promise<void> {
    this.entity.dprPath = value.fsPath;
    if (save) {
      await Runtime.projects.treeView.save();
    }
  }

  async setDpk(value: Uri, save: boolean = false): Promise<void> {
    this.entity.dpkPath = value.fsPath;
    if (save) {
      await Runtime.projects.treeView.save();
    }
  }

  async setExecutable(value: Uri, save: boolean = false): Promise<void> {
    this.entity.exePath = value.fsPath;
    if (save) {
      await Runtime.projects.treeView.save();
    }
  }

  async setIni(value: Uri, save: boolean = false): Promise<void> {
    this.entity.iniPath = value.fsPath;
    if (save) {
      await Runtime.projects.treeView.save();
    }
  }

  createChild(
    type: DelphiProjectTreeItemType,
    children: DelphiProjectTreeItem[]
  ): void {
    let item: DelphiProjectTreeItem | undefined = undefined;
    switch (type) {
      case DelphiProjectTreeItemType.DprojFile: {
        if (this.projectDproj && fileExists(this.projectDproj)) {
          item = new DprojFile(
            basename(this.projectDproj.fsPath),
            this.projectDproj,
            this.projectType,
          );
        }
        break;
      }
      case DelphiProjectTreeItemType.DprFile: {
        if (this.projectDpr && fileExists(this.projectDpr)) {
          item = new DprFile(
            basename(this.projectDpr.fsPath),
            this.projectDpr
          );
        }
        break;
      }
      case DelphiProjectTreeItemType.DpkFile: {
        if (this.projectDpk && fileExists(this.projectDpk)) {
          item = new DpkFile(
            basename(this.projectDpk.fsPath),
            this.projectDpk,
          );
        }
        break;
      }
      case DelphiProjectTreeItemType.ExecutableFile: {
        if (this.projectExe && fileExists(this.projectExe)) {
          item = new ExeFile(
            basename(this.projectExe.fsPath),
            this.projectExe,
          );
        }
        break;
      }
      case DelphiProjectTreeItemType.IniFile: {
        if (this.projectIni && fileExists(this.projectIni)) {
          item = new IniFile(
            basename(this.projectIni.fsPath),
            this.projectIni,
          );
        }
        break;
      }
    }
    if (item) {
      item.project = this;
      children.push(item);
    }
  }
}

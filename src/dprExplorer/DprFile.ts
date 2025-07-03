import { TreeItemCollapsibleState, ThemeIcon, Uri } from 'vscode';
import { DprTreeItem } from './DprTreeItem';

export class DprFile extends DprTreeItem {
  public dproj?: Uri;
  public executable?: Uri;

  constructor(
    label: string,
    resourceUri: Uri,
    collapsibleState: TreeItemCollapsibleState = TreeItemCollapsibleState.None
  ) {
    super(label, resourceUri, collapsibleState);
    this.command = {
      command: 'vscode.open',
      title: 'Open DPR File',
      arguments: [this.resourceUri]
    };
    this.iconPath = new ThemeIcon('file-code');
    this.contextValue = 'dprFile';
  }
}

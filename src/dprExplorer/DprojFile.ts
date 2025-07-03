import { TreeItemCollapsibleState, ThemeIcon, Uri } from 'vscode';
import { DprTreeItem } from './DprTreeItem';

export class DprojFile extends DprTreeItem {
  constructor(
    label: string,
    resourceUri: Uri
  ) {
    super(label, resourceUri, TreeItemCollapsibleState.None);
    this.command = {
      command: 'vscode.open',
      title: 'Open DPROJ File',
      arguments: [this.resourceUri]
    };
    this.iconPath = new ThemeIcon('gear');
    this.contextValue = 'dprojFile';
  }
}

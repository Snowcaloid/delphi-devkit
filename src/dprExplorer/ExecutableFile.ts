import { TreeItemCollapsibleState, ThemeIcon, Uri } from 'vscode';
import { DprTreeItem } from './DprTreeItem';

export class ExecutableFile extends DprTreeItem {
  constructor(
    label: string,
    resourceUri: Uri
  ) {
    super(label, resourceUri, TreeItemCollapsibleState.None);
    this.command = {
      command: 'delphi-utils.launchExecutable',
      title: 'Launch Application',
      arguments: [this.resourceUri]
    };
    this.iconPath = new ThemeIcon('play-circle');
    this.contextValue = 'executableFile';
  }
}

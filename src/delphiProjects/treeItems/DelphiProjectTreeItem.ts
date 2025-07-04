import { TreeItem, TreeItemCollapsibleState, Uri } from 'vscode';

export abstract class DelphiProjectTreeItem extends TreeItem {
  constructor(
    public readonly label: string,
    public readonly resourceUri: Uri,
    collapsibleState: TreeItemCollapsibleState = TreeItemCollapsibleState.None
  ) {
    super(label, collapsibleState);
    this.tooltip = this.resourceUri.fsPath;
  }
}

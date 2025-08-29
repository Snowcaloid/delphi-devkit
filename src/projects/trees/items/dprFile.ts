import { ThemeIcon, Uri } from 'vscode';
import { BaseFileItem, MainProjectItem } from './baseFile';
import { DelphiProjectTreeItemType } from '../../../types';

export class DprFileItem extends BaseFileItem {
  constructor(project: MainProjectItem, label: string, resourceUri: Uri) {
    super(DelphiProjectTreeItemType.DprFile, label, resourceUri, project);
    this.command = {
      command: 'vscode.open',
      title: 'Open DPR File',
      arguments: [this.projectDpr]
    };
    this.iconPath = new ThemeIcon('file-code');
  }
}

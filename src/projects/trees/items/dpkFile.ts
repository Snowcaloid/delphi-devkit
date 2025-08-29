import { ThemeIcon, Uri } from 'vscode';
import { BaseFileItem, MainProjectItem } from './baseFile';
import { DelphiProjectTreeItemType } from '../../../types';

export class DpkFileItem extends BaseFileItem {
  constructor(project: MainProjectItem, label: string, resourceUri: Uri) {
    super(DelphiProjectTreeItemType.DpkFile, label, resourceUri, project);
    this.command = {
      command: 'vscode.open',
      title: 'Open DPK File',
      arguments: [this.projectDpk]
    };
    this.iconPath = new ThemeIcon('package');
  }
}

import { ThemeIcon, Uri } from 'vscode';
import { BaseFileItem, MainProjectItem } from './baseFile';
import { DelphiProjectTreeItemType } from '../../../types';

export class DprojFileItem extends BaseFileItem {
  constructor(project: MainProjectItem, label: string, resourceUri: Uri) {
    super(DelphiProjectTreeItemType.DprojFile, label, resourceUri, project);
    this.command = {
      command: 'vscode.open',
      title: 'Open DPROJ File',
      arguments: [this.projectDproj]
    };
    this.iconPath = new ThemeIcon('gear');
  }
}

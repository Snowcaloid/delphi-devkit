import { ThemeIcon, Uri } from 'vscode';
import { BaseFileItem, MainProjectItem } from './baseFile';
import { DelphiProjectTreeItemType } from '../../../types';
import { PROJECTS } from '../../../constants';

export class ExeFileItem extends BaseFileItem {
  constructor(project: MainProjectItem, label: string, resourceUri: Uri) {
    super(DelphiProjectTreeItemType.ExecutableFile, label, resourceUri, project);
    this.command = {
      command: PROJECTS.COMMAND.RUN_EXECUTABLE,
      title: 'Launch Application',
      arguments: [this]
    };
    this.iconPath = new ThemeIcon('run');
  }
}

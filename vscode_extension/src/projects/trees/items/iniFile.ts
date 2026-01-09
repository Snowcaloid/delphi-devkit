import { ThemeIcon, Uri } from 'vscode';
import { BaseFileItem, MainProjectItem } from './baseFile';
import { DelphiProjectTreeItemType } from '../../../types';
import { PROJECTS } from '../../../constants';

export class IniFileItem extends BaseFileItem {
  constructor(project: MainProjectItem, label: string, resourceUri: Uri) {
    super(DelphiProjectTreeItemType.IniFile, label, resourceUri, project);
    this.command = {
      command: PROJECTS.COMMAND.CONFIGURE_OR_CREATE_INI,
      title: 'Open INI File',
      arguments: [this]
    };
    this.iconPath = new ThemeIcon('settings');
  }
}

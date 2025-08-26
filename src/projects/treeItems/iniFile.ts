import { ThemeIcon, Uri } from 'vscode';
import { DelphiProjectTreeItem } from './delphiProjectTreeItem';
import { DelphiProjectTreeItemType } from '../../types';
import { ProjectType } from './delphiProject';
import { Projects } from '../../constants';

export class IniFile extends DelphiProjectTreeItem {
  constructor(
    label: string,
    resourceUri: Uri,
  ) {
    super(DelphiProjectTreeItemType.IniFile, label, resourceUri, ProjectType.Application);
    this.command = {
      command: Projects.Command.ConfigureOrCreateIni,
      title: 'Open INI File',
      arguments: [this.projectIni]
    };
    this.iconPath = new ThemeIcon('settings');
    this.contextValue = 'iniFile';
  }
}

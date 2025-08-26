import { ThemeIcon, Uri } from 'vscode';
import { DelphiProjectTreeItem } from './delphiProjectTreeItem';
import { DelphiProjectTreeItemType } from '../../types';
import { ProjectType } from './delphiProject';
import { Projects } from '../../constants';

export class ExeFile extends DelphiProjectTreeItem {
  constructor(
    label: string,
    resourceUri: Uri,
  ) {
    super(DelphiProjectTreeItemType.ExecutableFile, label, resourceUri, ProjectType.Application);
    this.command = {
      command: Projects.Command.RunExecutable,
      title: 'Launch Application',
      arguments: [this.projectExe]
    };
    this.iconPath = new ThemeIcon('run');
    this.contextValue = 'executableFile';
  }
}

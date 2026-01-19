import { Feature } from '../types';
import { CompilerPicker } from './compiler/statusBar';
import { GroupProjectTreeView, WorkspacesTreeView } from './trees/treeView';
import { ProjectsCommands } from './commands';
import { GroupProjectPicker } from './pickers/groupProjPicker';

export class ProjectsFeature implements Feature {
  public workspacesTreeView: WorkspacesTreeView = new WorkspacesTreeView();
  public groupProjectTreeView: GroupProjectTreeView = new GroupProjectTreeView();
  public compilerStatusBarItem: CompilerPicker = new CompilerPicker();
  public groupProjectPicker: GroupProjectPicker = new GroupProjectPicker();

  public async initialize(): Promise<void> {
    ProjectsCommands.register();
  }
}

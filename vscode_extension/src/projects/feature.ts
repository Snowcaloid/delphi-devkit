import { Feature } from '../types';
import { Compiler } from './compiler/compiler';
import { CompilerPicker } from './compiler/statusBar';
import { GroupProjectTreeView, WorkspacesTreeView } from './trees/treeView';
import { ProjectsCommands } from './commands';
import { GroupProjectPicker } from './pickers/groupProjPicker';
import { Entities } from './entities';

export class ProjectsFeature implements Feature {
  public workspacesTreeView: WorkspacesTreeView = new WorkspacesTreeView();
  public groupProjectTreeView: GroupProjectTreeView = new GroupProjectTreeView();
  public compiler: Compiler = new Compiler();
  public compilerStatusBarItem: CompilerPicker = new CompilerPicker();
  public groupProjectPicker: GroupProjectPicker = new GroupProjectPicker();

  public async initialize(): Promise<void> {
    ProjectsCommands.register();
  }

  public isCurrentlyCompiling(project: Entities.Project): boolean {
    return this.compiler.currentlyCompilingProjectId === project.id;
  }
}

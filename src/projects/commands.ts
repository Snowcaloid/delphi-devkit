import { commands, env, Uri, window } from "vscode";
import { Runtime } from "../runtime";
import { Projects } from "../constants";
import { Coroutine } from "../typings";
import { ProjectEntity } from "../db/entities";


export class ProjectCommands {
  public static register() {
    Runtime.extension.subscriptions.push(...[
      commands.registerCommand(Projects.Command.CompileSelectedProject, ProjectCommands.compileSelectedProject),
      commands.registerCommand(Projects.Command.RecreateSelectedProject, ProjectCommands.recreateSelectedProject),
      commands.registerCommand(Projects.Command.RunSelectedProject, ProjectCommands.runSelectedProject),
    ]);
  }

  private static async selectedProjectAction(callback: Coroutine<void, [ProjectEntity]>): Promise<void> {
    const workspace = await Runtime.db.getWorkspace();
    if (!workspace?.currentProject) { return; }
    await callback(workspace.currentProject);
  }

  private static async compileSelectedProject() {
    await ProjectCommands.selectedProjectAction(async (project) => {
      const path = project.dprojPath || project.dprPath || project.dpkPath;
      if (!path) { return; }
      Runtime.compiler.compile(Uri.file(path), false);
    });
  }

  private static async recreateSelectedProject() {
    await ProjectCommands.selectedProjectAction(async (project) => {
      const path = project.dprojPath || project.dprPath || project.dpkPath;
      if (!path) { return; }
      Runtime.compiler.compile(Uri.file(path), true);
    });
  }

  private static async runSelectedProject() {
    await ProjectCommands.selectedProjectAction(async (project) => {
      if (!project.exePath) { return; }
      try {
        // Use the system's default application handler to launch the executable
        await env.openExternal(Uri.file(project.exePath));
      } catch (error) {
        window.showErrorMessage(`Failed to launch executable: ${error}`);
      }
    });
  }
}
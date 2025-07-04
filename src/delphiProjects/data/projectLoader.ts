import { Uri, workspace } from 'vscode';
import { DelphiProject, ProjectType } from '../treeItems/DelphiProject';
import { ProjectData } from '../types';

/**
 * Service for loading projects from cached configuration data.
 */
export class ProjectLoader {

  /**
   * Load projects from cached configuration data.
   */
  static async loadProjectsFromConfig(configData: any): Promise<DelphiProject[] | null> {
    if (!configData || !configData.defaultProjects) {
      return null;
    }

    const projects: DelphiProject[] = [];

    for (const projectData of configData.defaultProjects) {
      try {
        // Verify at least one main file exists
        const mainFileExists = await this.verifyMainFileExists(projectData);
        if (!mainFileExists) {
          continue; // Skip this project as no main files exist
        }

        const project = new DelphiProject(projectData.name, projectData.type || ProjectType.Application);

        // Restore file references if they exist
        await this.restoreFileReferences(project, projectData);

        project.updateCollapsibleState();
        projects.push(project);
      } catch {
        // Project data is invalid, skip it
        continue;
      }
    }

    return projects;
  }

  /**
   * Verify that at least one main project file exists.
   */
  private static async verifyMainFileExists(projectData: ProjectData): Promise<boolean> {
    // Check DPROJ file
    if (projectData.hasDproj && projectData.dprojAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.dprojAbsolutePath));
        return true;
      } catch {
        // DPROJ file no longer exists
      }
    }

    // Check DPR file
    if (projectData.hasDpr && projectData.dprAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.dprAbsolutePath));
        return true;
      } catch {
        // DPR file no longer exists
      }
    }

    // Check DPK file
    if (projectData.hasDpk && projectData.dpkAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.dpkAbsolutePath));
        return true;
      } catch {
        // DPK file no longer exists
      }
    }

    return false;
  }

  /**
   * Restore file references from cached data to project instance.
   */
  private static async restoreFileReferences(project: DelphiProject, projectData: ProjectData): Promise<void> {
    // Restore DPROJ file reference
    if (projectData.hasDproj && projectData.dprojAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.dprojAbsolutePath));
        project.dproj = Uri.file(projectData.dprojAbsolutePath);
      } catch {
        // File no longer exists
      }
    }

    // Restore DPR file reference
    if (projectData.hasDpr && projectData.dprAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.dprAbsolutePath));
        project.dpr = Uri.file(projectData.dprAbsolutePath);
      } catch {
        // File no longer exists
      }
    }

    // Restore DPK file reference
    if (projectData.hasDpk && projectData.dpkAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.dpkAbsolutePath));
        project.dpk = Uri.file(projectData.dpkAbsolutePath);
      } catch {
        // File no longer exists
      }
    }

    // Restore executable file reference
    if (projectData.hasExecutable && projectData.executableAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.executableAbsolutePath));
        project.executable = Uri.file(projectData.executableAbsolutePath);
      } catch {
        // File no longer exists
      }
    }

    // Restore INI file reference
    if (projectData.hasIni && projectData.iniAbsolutePath) {
      try {
        await workspace.fs.stat(Uri.file(projectData.iniAbsolutePath));
        project.ini = Uri.file(projectData.iniAbsolutePath);
      } catch {
        // File no longer exists
      }
    }
  }
}

import { Uri, workspace, RelativePattern } from 'vscode';
import { basename, dirname, join } from 'path';
import { promises as fs } from 'fs';
import { DelphiProject, ProjectType } from '../treeItems/DelphiProject';
import { DelphiProjectUtils } from '../utils';

/**
 * Project discovery service for finding Delphi projects in the workspace.
 */
export class ProjectDiscovery {

  /**
   * Discover all Delphi projects in the workspace based on configuration.
   */
  static async getAllProjects(): Promise<DelphiProject[]> {
    console.log('ProjectDiscovery: Starting getAllProjects...');

    if (!workspace.workspaceFolders) {
      console.log('ProjectDiscovery: No workspace folders found');
      return [];
    }

    console.log(`ProjectDiscovery: Found ${workspace.workspaceFolders.length} workspace folders`);

    const projectMap = new Map<string, DelphiProject>(); // Key: project base name + directory

    // Get configuration
    const config = workspace.getConfiguration('delphi-utils.delphiProjects');
    const projectPaths: string[] = config.get('projectPaths', ['**']);
    const excludePatterns: string[] = config.get('excludePatterns', []);

    console.log('ProjectDiscovery: Project paths:', projectPaths);
    console.log('ProjectDiscovery: Exclude patterns:', excludePatterns);

    for (const folder of workspace.workspaceFolders) {
      console.log(`ProjectDiscovery: Processing folder: ${folder.uri.fsPath}`);

      // Create exclude pattern for workspace.findFiles
      const excludeGlob = excludePatterns.length > 0 ? `{${excludePatterns.join(',')}}` : undefined;
      console.log('ProjectDiscovery: Using exclude glob:', excludeGlob);

      // Process DPROJ files first (modern projects)
      await this.processDprojFiles(folder, projectPaths, excludeGlob, projectMap);

      // Process standalone DPR files (legacy projects)
      await this.processStandaloneDprFiles(folder, projectPaths, excludeGlob, projectMap);

      // Process standalone DPK files (legacy packages)
      await this.processStandaloneDpkFiles(folder, projectPaths, excludeGlob, projectMap);
    }

    console.log(`ProjectDiscovery: Finished processing, found ${projectMap.size} total projects`);
    return Array.from(projectMap.values());
  }

  /**
   * Process DPROJ files to create modern Delphi projects.
   */
  private static async processDprojFiles(
    folder: any,
    projectPaths: string[],
    excludeGlob: string | undefined,
    projectMap: Map<string, DelphiProject>
  ): Promise<void> {
    console.log('ProjectDiscovery: Searching for DPROJ files...');
    let allDprojFiles: Uri[] = [];

    for (const projectPath of projectPaths) {
      const dprojPattern = new RelativePattern(folder, `${projectPath}/*.[Dd][Pp][Rr][Oo][Jj]`);
      const dprojFiles = await workspace.findFiles(dprojPattern, excludeGlob);
      allDprojFiles.push(...dprojFiles);
    }

    console.log(`ProjectDiscovery: Found ${allDprojFiles.length} DPROJ files after filtering`);

    for (const dprojFile of allDprojFiles) {
      const fileName = basename(dprojFile.fsPath);
      const baseName = fileName.replace(/\.[^/.]+$/, "");
      const dirPath = dirname(dprojFile.fsPath);
      const projectKey = `${baseName}-${dirPath}`;

      // Determine project type by checking for DPK vs DPR
      let projectType = ProjectType.Application;

      // Check for DPK file (package) in the same project paths
      const correspondingDpk = await this.findCorrespondingFile(
        folder, projectPaths, baseName, dirPath, 'Dd][Pp][Kk', excludeGlob
      );
      if (correspondingDpk) {
        projectType = ProjectType.Package;
      }

      const project = new DelphiProject(baseName, projectType);
      project.dproj = dprojFile;

      // Add DPK file if found
      if (correspondingDpk) {
        project.dpk = correspondingDpk;
      }

      // Look for corresponding DPR file
      const correspondingDpr = await this.findCorrespondingFile(
        folder, projectPaths, baseName, dirPath, 'Dd][Pp][Rr', excludeGlob
      );
      if (correspondingDpr) {
        project.dpr = correspondingDpr;
      }

      // Try to parse executable path from DPROJ
      await this.processExecutableFromDproj(project, dprojFile, baseName);

      project.updateCollapsibleState();
      projectMap.set(projectKey, project);
      console.log(`ProjectDiscovery: Added DPROJ project: ${baseName}`);
    }
  }

  /**
   * Process standalone DPR files (legacy projects without DPROJ).
   */
  private static async processStandaloneDprFiles(
    folder: any,
    projectPaths: string[],
    excludeGlob: string | undefined,
    projectMap: Map<string, DelphiProject>
  ): Promise<void> {
    console.log('ProjectDiscovery: Searching for standalone DPR files...');
    let allDprFiles: Uri[] = [];

    for (const projectPath of projectPaths) {
      const dprPattern = new RelativePattern(folder, `${projectPath}/*.[Dd][Pp][Rr]`);
      const dprFiles = await workspace.findFiles(dprPattern, excludeGlob);
      allDprFiles.push(...dprFiles);
    }

    console.log(`ProjectDiscovery: Found ${allDprFiles.length} DPR files after filtering`);

    for (const dprFile of allDprFiles) {
      const fileName = basename(dprFile.fsPath);
      const baseName = fileName.replace(/\.[^/.]+$/, "");
      const dirPath = dirname(dprFile.fsPath);
      const projectKey = `${baseName}-${dirPath}`;

      // Only add if we don't already have a project with DPROJ
      if (!projectMap.has(projectKey)) {
        const project = new DelphiProject(baseName, ProjectType.Application);
        project.dpr = dprFile;
        project.updateCollapsibleState();
        projectMap.set(projectKey, project);
      }
    }
  }

  /**
   * Process standalone DPK files (legacy packages without DPROJ).
   */
  private static async processStandaloneDpkFiles(
    folder: any,
    projectPaths: string[],
    excludeGlob: string | undefined,
    projectMap: Map<string, DelphiProject>
  ): Promise<void> {
    console.log('ProjectDiscovery: Searching for standalone DPK files...');
    let allDpkFiles: Uri[] = [];

    for (const projectPath of projectPaths) {
      const dpkPattern = new RelativePattern(folder, `${projectPath}/*.[Dd][Pp][Kk]`);
      const dpkFiles = await workspace.findFiles(dpkPattern, excludeGlob);
      allDpkFiles.push(...dpkFiles);
    }

    console.log(`ProjectDiscovery: Found ${allDpkFiles.length} DPK files after filtering`);

    for (const dpkFile of allDpkFiles) {
      const fileName = basename(dpkFile.fsPath);
      const baseName = fileName.replace(/\.[^/.]+$/, "");
      const dirPath = dirname(dpkFile.fsPath);
      const projectKey = `${baseName}-${dirPath}`;

      // Only add if we don't already have a project
      if (!projectMap.has(projectKey)) {
        const project = new DelphiProject(baseName, ProjectType.Package);
        project.dpk = dpkFile;
        project.updateCollapsibleState();
        projectMap.set(projectKey, project);
      }
    }
  }

  /**
   * Find a corresponding file (DPR, DPK) for a given project.
   */
  private static async findCorrespondingFile(
    folder: any,
    projectPaths: string[],
    baseName: string,
    dirPath: string,
    fileExtPattern: string,
    excludeGlob: string | undefined
  ): Promise<Uri | undefined> {
    for (const projectPath of projectPaths) {
      const pattern = new RelativePattern(folder, `${projectPath}/${baseName}.[${fileExtPattern}]`);
      const files = await workspace.findFiles(pattern, excludeGlob);
      const correspondingFile = files.find(file => dirname(file.fsPath) === dirPath);
      if (correspondingFile) {
        return correspondingFile;
      }
    }
    return undefined;
  }

  /**
   * Process executable information from DPROJ file.
   */
  private static async processExecutableFromDproj(
    project: DelphiProject,
    dprojFile: Uri,
    baseName: string
  ): Promise<void> {
    try {
      console.log(`ProjectDiscovery: Parsing executable from ${baseName}.dproj...`);
      const executableUri = await DelphiProjectUtils.findExecutableFromDproj(dprojFile);
      if (executableUri) {
        project.executable = executableUri;
        console.log(`ProjectDiscovery: Found executable: ${executableUri.fsPath}`);

        // Look for corresponding INI file next to the executable
        const executableDir = dirname(executableUri.fsPath);
        const executableName = basename(executableUri.fsPath).replace(/\.[^/.]+$/, "");
        const iniPath = join(executableDir, `${executableName}.ini`);

        try {
          await fs.access(iniPath);
          project.ini = Uri.file(iniPath);
          console.log(`ProjectDiscovery: Found INI file: ${iniPath}`);
        } catch {
          // INI file doesn't exist, that's fine
        }
      } else {
        console.log(`ProjectDiscovery: No executable found in ${baseName}.dproj`);
      }
    } catch (error) {
      console.error(`ProjectDiscovery: Failed to parse executable from DPROJ (${baseName}):`, error);
    }
  }
}

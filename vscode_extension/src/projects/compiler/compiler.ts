import { Uri, workspace, window, languages, OutputChannel } from 'vscode';
import { basename, dirname, join } from 'path';
import { spawn } from 'child_process';
import { Runtime } from '../../runtime';
import { assertError, fileExists } from '../../utils';
import { CompilerOutputDefinitionProvider } from './language';
import { PROJECTS } from '../../constants';
import { Entities } from '../entities';

export class Compiler {
  private outputChannel: OutputChannel = window.createOutputChannel('Delphi Compiler', PROJECTS.LANGUAGES.COMPILER);
  private linkProvider: CompilerOutputDefinitionProvider = new CompilerOutputDefinitionProvider();
  public currentlyCompilingProjectId: number = -1;

  constructor() {
    Runtime.extension.subscriptions.push(
      ...[
        this.outputChannel,
        languages.registerDocumentLinkProvider({ language: PROJECTS.LANGUAGES.COMPILER }, this.linkProvider),
      ]
    );
  }

  public async compileWorkspaceItem(link: Entities.ProjectLink, recreate: boolean = false): Promise<boolean> {
    const project = link.project;
    if (!assertError(project, 'Cannot determine project for project link.')) return false;
    const path = project!.dproj || project!.dpr || project!.dpk;
    if (!assertError(path, 'No suitable project file (DPROJ, DPR, DPK) found to compile.')) return false;

    const fileUri = Uri.file(path!);
    const ws = link.workspace;
    if (!assertError(ws, 'Cannot determine workspace for project.')) return false;
    const compiler = ws!.compiler;
    if (!assertError(compiler, 'Unable to determine compiler configuration for workspace.')) return false;
    return await this.compile(link, fileUri, compiler!, recreate);
  }

  public async compileGroupProjectItem(link: Entities.ProjectLink, recreate: boolean = false): Promise<boolean> {
    const project = link.project;
    if (!assertError(project, 'Cannot determine project for project link.')) return false;
    const path = project!.dproj || project!.dpr || project!.dpk;
    if (!assertError(path, 'No suitable project file (DPROJ, DPR, DPK) found to compile.')) return false;
    const fileUri = Uri.file(path!);
    const groupProject = link.groupProject;
    if (!assertError(groupProject, 'Cannot determine group project for project.')) return false;
    const compiler = groupProject!.compiler;
    if (!assertError(compiler, 'Unable to determine compiler configuration for group project.')) return false;

    return await this.compile(link, fileUri, compiler!, recreate);
  }

  private async compile(
    link: Entities.ProjectLink,
    file: Uri,
    compilerConfiguration: Entities.CompilerConfiguration,
    recreate: boolean = false
  ): Promise<boolean> {
    // Use OutputChannel and diagnostics
    const project = link.project;
    if (!assertError(project, 'Cannot determine project for project link.')) return false;
    this.currentlyCompilingProjectId = project!.id;
    try {
      if (!assertError(fileExists(file), `Project file not found: ${file.fsPath}`)) return false;
      const fileName = basename(file.fsPath);
      const projectDir = dirname(file.fsPath);
      const relativePath = workspace.asRelativePath(projectDir);
      const pathDescription = relativePath === projectDir ? projectDir : relativePath;
      const actionDescription = recreate ? 'recreate (clean + build)' : 'compile (clean + make)';
      const buildTarget = recreate ? 'Build' : 'Make';
      const buildArguments = [`/t:Clean,${buildTarget}`, ...compilerConfiguration.build_arguments];
      // Use extension path to find the script
      const scriptPath = Runtime.extension.asAbsolutePath(join('dist', 'compile.ps1'));
      const buildArgumentsString = buildArguments.join(' ');
      let psArgs = [
        '-ExecutionPolicy',
        'Bypass',
        '-File',
        scriptPath,
        '-ProjectPath',
        file.fsPath,
        '-RSVarsPath',
        '<TODO>',
        '-MSBuildPath',
        '<TODO>',
        '-FileName',
        fileName,
        '-ActionDescription',
        actionDescription,
        '-PathDescription',
        pathDescription,
        '-BuildArguments',
        buildArgumentsString,
        '-CompilerName',
        '<TODO>'
      ];

      await workspace.getConfiguration('output.smartScroll').update('enabled', false);
      this.linkProvider.compilerIsActive = true;
      window.showInformationMessage(`Starting ${actionDescription} for ${fileName} using <TODO>...`);
      this.outputChannel.clear();
      this.outputChannel.show(true);
      // Run PowerShell script and capture output
      const proc = spawn('powershell.exe', psArgs, {
        stdio: ['pipe', 'pipe', 'pipe'], // Explicit stdio configuration
        windowsHide: true // Hide PowerShell window
      });
      let output = '';
      const handleIO = (data: Buffer) => {
        const text = data.toString('utf8');
        this.outputChannel.append(text);
        output += text;
      };
      proc.stdout.on('data', handleIO);
      proc.stderr.on('data', handleIO);
      proc.on('close', async (code: number) => {
        this.linkProvider.compilerIsActive = false;
        this.outputChannel.show(true);
        if (code === 0) window.showInformationMessage('Build succeeded');
        else window.showErrorMessage('Build failed');
        return code === 0;
      });
    } catch (error) {
      window.showErrorMessage(`Failed to ${recreate ? 'recreate' : 'compile'} project: ${error}`);
      return false;
    } finally {
      this.currentlyCompilingProjectId = -1;
    }
    return false;
  }
}

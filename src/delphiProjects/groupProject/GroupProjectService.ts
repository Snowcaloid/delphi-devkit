import { window, workspace, commands, Uri, ProgressLocation } from 'vscode';
import { ProjectCacheManager } from '../data/cacheManager';
import { GroupProjParser } from './GroupProjParser';
import { DelphiProjectUtils } from '../utils';
import { ProjectData } from '../types';
import * as path from 'path';
import * as fs from 'fs';
import { getExpectedExePathFromDproj } from '../utils/getExpectedExePathFromDproj';

export class GroupProjectService {
  static async pickGroupProject(delphiProjectsProvider: { refresh: () => void }) {
    // Scan for .groupproj files
    const groupProjUris = await window.withProgress({
      location: ProgressLocation.Notification,
      title: 'Scanning for .groupproj files...',
      cancellable: false
    }, async () => {
      return await workspace.findFiles('**/*.groupproj');
    });
    if (!groupProjUris.length) {
      window.showInformationMessage('No .groupproj files found in the workspace.');
      return;
    }
    // Show QuickPick
    const picked = await window.showQuickPick(
      groupProjUris.map(uri => ({
        label: workspace.asRelativePath(uri),
        uri
      })),
      { placeHolder: 'Select a .groupproj file to load' }
    );
    if (!picked) { return; }
    // Parse .groupproj for dproj paths
    const parsed = await GroupProjParser.parseGroupProjFile(picked.uri);
    // For each dproj, build full ProjectData
    const projects: ProjectData[] = [];
    for (const dprojPath of parsed.dprojPaths) {
      let absPath: Uri;
      if (path.isAbsolute(dprojPath)) {
        absPath = Uri.file(dprojPath);
      } else {
        // Resolve relative to the .groupproj file's directory
        const groupProjDir = path.dirname(picked.uri.fsPath);
        absPath = Uri.file(path.resolve(groupProjDir, dprojPath));
      }
      // Check if file exists
      try {
        await workspace.fs.stat(absPath);
      } catch { continue; }
      // Find related files
      const dprUri = await DelphiProjectUtils.findDprFromDproj(absPath);
      const dpkUri = await DelphiProjectUtils.findDpkFromDproj(absPath);
      // --- Executable detection: use getExpectedExePathFromDproj first ---
      let exeUri: Uri | null = null;
      if (absPath && dprUri) {
        const expectedExePath = await getExpectedExePathFromDproj(absPath.fsPath, dprUri.fsPath);
        if (expectedExePath && fs.existsSync(expectedExePath)) {
          exeUri = Uri.file(expectedExePath);
        }
      }
      // Fallback to old method if not found
      if (!exeUri) {
        exeUri = await DelphiProjectUtils.findExecutableFromDproj(absPath);
      }
      // Find INI file matching the executable name in the executable's directory (like projectDiscovery)
      let iniUri: Uri | undefined = undefined;
      if (exeUri) {
        const executableDir = path.dirname(exeUri.fsPath);
        const executableName = path.basename(exeUri.fsPath, path.extname(exeUri.fsPath));
        const iniPath = path.join(executableDir, `${executableName}.ini`);
        try {
          await fs.promises.access(iniPath);
          iniUri = Uri.file(iniPath);
        } catch {}
      }
      // If not found, fallback to any .ini in the dproj directory (legacy fallback)
      if (!iniUri) {
        try {
          const dprojDir = path.dirname(absPath.fsPath);
          const files: string[] = await fs.promises.readdir(dprojDir);
          const iniFile = files.find((f: string) => f.toLowerCase().endsWith('.ini'));
          if (iniFile) {
            iniUri = Uri.file(path.join(dprojDir, iniFile));
          }
        } catch {}
      }
      // Compose ProjectData
      projects.push({
        name: absPath.path.split(/[\\\/]/).pop()?.replace(/\.[^.]+$/, '') || absPath.path,
        type: dpkUri ? 'package' : 'application',
        hasDproj: true,
        dprojPath: dprojPath,
        dprojAbsolutePath: absPath.fsPath,
        hasDpr: !!dprUri,
        dprPath: dprUri ? workspace.asRelativePath(dprUri) : undefined,
        dprAbsolutePath: dprUri?.fsPath,
        hasDpk: !!dpkUri,
        dpkPath: dpkUri ? workspace.asRelativePath(dpkUri) : undefined,
        dpkAbsolutePath: dpkUri?.fsPath,
        hasExecutable: !!exeUri,
        executablePath: exeUri ? workspace.asRelativePath(exeUri) : undefined,
        executableAbsolutePath: exeUri?.fsPath,
        hasIni: !!iniUri,
        iniPath: iniUri ? workspace.asRelativePath(iniUri) : undefined,
        iniAbsolutePath: iniUri?.fsPath
      });
    }
    // Save to cache
    const cacheManager = new ProjectCacheManager();
    const cache = await cacheManager.loadCacheData() || { lastUpdated: '', version: '1.0', defaultProjects: [] };
    cache.currentGroupProject = {
      groupProjPath: picked.label,
      groupProjAbsolutePath: picked.uri.fsPath,
      name: picked.label,
      projects
    };
    cache.lastUpdated = new Date().toISOString();
    await cacheManager.saveCacheData(cache);
    // Set context for UI
    await commands.executeCommand('setContext', 'delphiUtils:groupProjectLoaded', true);
    delphiProjectsProvider.refresh();
    window.showInformationMessage(`Loaded group project: ${picked.label}`);
  }

  static async unloadGroupProject(delphiProjectsProvider: { refresh: () => void }) {
    const cacheManager = new ProjectCacheManager();
    const cache = await cacheManager.loadCacheData();
    if (cache && cache.currentGroupProject) {
      delete cache.currentGroupProject;
      cache.lastUpdated = new Date().toISOString();
      await cacheManager.saveCacheData(cache);
    }
    await commands.executeCommand('setContext', 'delphiUtils:groupProjectLoaded', false);
    delphiProjectsProvider.refresh();
    window.showInformationMessage('Unloaded group project. Showing default projects.');
  }
}

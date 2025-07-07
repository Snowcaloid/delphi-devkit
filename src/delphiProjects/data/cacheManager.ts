import { Uri, workspace } from 'vscode';
import { join } from 'path';
import { promises as fs } from 'fs';
import { ProjectCacheData } from '../types';

/**
 * Utility class for managing the project cache file operations.
 */
export class ProjectCacheManager {
  private configFileName = 'cache.json';
  private delphiDirName = '.delphi';

  /**
   * Get the path to the cache configuration file.
   */
  async getConfigFilePath(): Promise<string | null> {
    if (!workspace.workspaceFolders || workspace.workspaceFolders.length === 0) {
      return null;
    }

    const workspaceRoot = workspace.workspaceFolders[0].uri.fsPath;
    const delphiDir = join(workspaceRoot, '.vscode', this.delphiDirName);

    // Ensure .vscode/.delphi directory exists
    try {
      await fs.access(delphiDir);
    } catch {
      await fs.mkdir(delphiDir, { recursive: true });
    }

    return join(delphiDir, this.configFileName);
  }

  /**
   * Load project cache data from the configuration file.
   */
  async loadCacheData(): Promise<ProjectCacheData | null> {
    const configPath = await this.getConfigFilePath();
    if (!configPath) {
      return null;
    }

    try {
      const configContent = await fs.readFile(configPath, 'utf8');
      return JSON.parse(configContent);
    } catch {
      // Config file doesn't exist or is invalid, return null
      return null;
    }
  }

  /**
   * Save project cache data to the configuration file.
   */
  async saveCacheData(cacheData: ProjectCacheData): Promise<void> {
    const configPath = await this.getConfigFilePath();
    if (!configPath) {
      return;
    }

    try {
      await fs.writeFile(configPath, JSON.stringify(cacheData, null, 2), 'utf8');
    } catch (error) {
      console.error('Failed to save Delphi projects to config:', error);
    }
  }
}

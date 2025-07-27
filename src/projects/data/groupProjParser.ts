import { Uri } from 'vscode';
import { promises as fs } from 'fs';
import { dirname } from 'path';
import { fileExists } from '../../utils';


export class GroupProjParser {
  public async getDprojs(groupProjUri: Uri): Promise<Uri[]> {
    const content = await fs.readFile(groupProjUri.fsPath, 'utf8');
    // Simple regex to extract all <Projects Include="..."> tags
    const projectRegex = /<Projects\s+Include="([^"]+)"/gi;
    const dprojPaths: Uri[] = [];
    let match;
    while ((match = projectRegex.exec(content))) {
      const relPath = match[1];
      if (relPath.toLowerCase().endsWith('.dproj')) {
        const absolutePath = Uri.joinPath(Uri.file(dirname(groupProjUri.fsPath)), relPath);
        if (
          !dprojPaths.find(p => p.fsPath === absolutePath.fsPath) && 
          fileExists(absolutePath)
        ) {
          dprojPaths.push(absolutePath);
        }
      }
    }
    return dprojPaths;
  }
}

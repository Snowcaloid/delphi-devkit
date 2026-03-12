import { CancellationToken, FileDecoration, FileDecorationProvider, ThemeColor, Uri } from 'vscode';
import { PROJECTS } from '../../constants';
import { fileExists } from '../../utils';

export class TreeItemDecorator implements FileDecorationProvider {
  public async provideFileDecoration(uri: Uri, token: CancellationToken): Promise<FileDecoration | undefined> {
    let decoration: FileDecoration;
    switch (uri.scheme) {
      case PROJECTS.SCHEME.SELECTED:
        decoration = new FileDecoration('←S', 'selected project for compiling shortcuts', new ThemeColor('list.focusHighlightForeground'));
        decoration.propagate = false;
        return decoration;
      case PROJECTS.SCHEME.MISSING:
        decoration = new FileDecoration('!', 'file does not exist', new ThemeColor('errorForeground'));
        decoration.propagate = false;
        return decoration;
      case PROJECTS.SCHEME.COMPILING:
        decoration = new FileDecoration('●', 'currently being compiled', new ThemeColor('charts.yellow'));
        decoration.propagate = false;
        const iv = setInterval(() => {
          if (uri.scheme !== PROJECTS.SCHEME.COMPILING) clearInterval(iv);
          decoration.badge = decoration.badge === '●' ? '○' : '●';
        }, 1000);
        return decoration;
      case 'file':
        if (!await fileExists(uri)) {
          decoration = new FileDecoration('!', 'file does not exist', new ThemeColor('errorForeground'));
          decoration.propagate = false;
          return decoration;
        }
    }
  }
}

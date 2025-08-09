import { CancellationToken, FileDecoration, FileDecorationProvider, ProviderResult, ThemeColor, Uri } from "vscode";
import { Projects } from "../../constants";

export class SelectedItemDecorator implements FileDecorationProvider {
	public provideFileDecoration(uri: Uri, token: CancellationToken): ProviderResult<FileDecoration> {
    if (uri.scheme !== Projects.Scheme.Selected) { return; }
		const decoration = new FileDecoration(
			"←S", 
			"selected project for compiling shortcuts", 
			new ThemeColor('list.focusHighlightForeground')
		);
		decoration.propagate = true;
		return decoration;
	}
}
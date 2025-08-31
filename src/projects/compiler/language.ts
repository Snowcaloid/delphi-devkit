import { CancellationToken, DocumentLink, DocumentLinkProvider, TextDocument, Range, Position, Uri, workspace } from "vscode";
import { fileExists } from "../../utils";

export namespace CompilerOutputLanguage {
  //  1____  2____  3__________________       4_   5_______________________
  // [ERROR][E1234] C:\Path\To\File.pas (line 42): Description of the error
  export const PATTERN = /^\[([^\]]+)\]\[([^\]]+)\] (.*?):(\d+) - (.*)$/;
  export const CONTENT = 0;
  export const SEVERITY = 1;
  export const CODE = 2;
  export const FILE = 3;
  export const LINE = 4;
  export const MESSAGE = 5;

  export const CODE_URL = 'https://docwiki.embarcadero.com/RADStudio/index.php?search=Delphi+';
}

export class CompilerOutputDefinitionProvider implements DocumentLinkProvider {
  public compilerIsActive: boolean = false;

  // Called by outputChannel.Show()
  public async provideDocumentLinks(
    document: TextDocument,
    token: CancellationToken
  ): Promise<DocumentLink[]> {
    if (this.compilerIsActive) return []; // Don't provide links while compiler is running
    const text = document.getText();
    let lines = text.split(/\r?\n/g);
    const matches = (
      await Promise.all(
        lines.map(line => line.match(CompilerOutputLanguage.PATTERN))
      )
    ).filter((match) => !!match);

    const matchesByFile = matches.reduce((acc, match) => {
      if (match) {
        const file = match[CompilerOutputLanguage.FILE];
        const existing = acc.find(item => item.file === file);
        if (existing) existing.matches.push(match);
        else acc.push({ file, matches: [match] });
      }
      return acc;
    }, [] as { file: string, matches: RegExpMatchArray[] }[]);

    return (await Promise.all(
      matchesByFile.map(async (o) => {
        const fileName = o.file;
        if (token.isCancellationRequested) throw new Error('Operation cancelled');
        if (!fileExists(fileName)) return [];
        const fileContent = await workspace.fs.readFile(Uri.file(fileName));
        const fileText = Buffer.from(fileContent).toString('utf8');
        const fileLines = fileText.split(/\r?\n/g);
        return o.matches.map((match) => {
          const line = match[0];
          const lineIndex = lines.indexOf(line);
          const code = match[CompilerOutputLanguage.CODE];
          const file = match[CompilerOutputLanguage.FILE];
          const lineNumText = match[CompilerOutputLanguage.LINE];
          const lineNum = parseInt(lineNumText, 10);
          const message = match[CompilerOutputLanguage.MESSAGE];
          const codeIndex = line.indexOf(code);
          const fileIndex = line.indexOf(file);

          let column = 1;
          const quotedString = message.match(/'(.*?)'/); // '%s' usually points to some symbol
          if (quotedString) {
            const quotedContent = quotedString ? quotedString[1] : '';
            const dotIndex = quotedContent.indexOf('.'); // if the quoted content is referencing Class.Member, slice to just Member
            const contentToFind = dotIndex > 0 ? quotedContent.slice(dotIndex + 1) : quotedContent;
            if (fileLines.length >= lineNum) {
              const targetLine = fileLines[lineNum - 1];
              column = Math.max(targetLine.indexOf(contentToFind) + 1, 1);
            }
          }

          const codeLink = new DocumentLink(
            new Range(
              new Position(lineIndex, codeIndex),
              new Position(lineIndex, codeIndex + code.length)),
            Uri.parse(`${CompilerOutputLanguage.CODE_URL}${code}`)
          );
          const fileLink = new DocumentLink(
            new Range(
              new Position(lineIndex, fileIndex),
              new Position(lineIndex, fileIndex + file.length + lineNumText.length + 1)),
            Uri.file(file).with({ fragment: `L${lineNum},${column}` })
          );
          return [fileLink, codeLink];
        });
      })
    )).flat(2);
  }
  public resolveDocumentLink(
    link: DocumentLink,
    token: CancellationToken
  ): undefined {} // Dont do anything with incomplete links
}
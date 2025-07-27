import { 
  DocumentLinkProvider, 
  TextDocument, 
  DocumentLink, 
  Range, 
  Uri, 
  Position 
} from 'vscode';
import { existsSync } from 'fs';
import { PROBLEMMATCHER_REGEX } from '.';
import { Runtime } from '../../runtime';

export class CompilerOutputLinkProvider implements DocumentLinkProvider {
  provideDocumentLinks(document: TextDocument): DocumentLink[] {
    const links: DocumentLink[] = [];
    const text = document.getText();
    const lines = text.split('\n');

    const problemRegex = PROBLEMMATCHER_REGEX[+!!Runtime.compiler.configuration?.usePrettyFormat];

    lines.forEach((line, lineIndex) => {
      const match = problemRegex.exec(line);
      if (match) {
        let filePath: string;
        let lineNum: number;
        
        if (Runtime.compiler.configuration?.usePrettyFormat) {
          filePath = match[3];
          lineNum = parseInt(match[4], 10);
        } else {
          filePath = match[1];
          lineNum = parseInt(match[2], 10);
        }
        
        if (existsSync(filePath)) {
          const startIndex = line.indexOf(filePath);
          const endIndex = startIndex + filePath.length;
          
          if (startIndex !== -1) {
            const range = new Range(
              new Position(lineIndex, startIndex),
              new Position(lineIndex, endIndex)
            );
            
            const uri = Uri.file(filePath).with({
              fragment: `L${lineNum}`
            });
            
            links.push(new DocumentLink(range, uri));
          }
        }
      }
    });

    return links;
  }
}
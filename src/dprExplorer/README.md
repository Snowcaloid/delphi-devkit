# DPR Explorer Module Structure

The DPR Explorer module has been refactored into a modular structure for maximum maintainability and clarity. Each class now has its own file, following the single responsibility principle.

## File Structure

```text
src/dprExplorer/
├── index.ts              # Export barrel for clean imports
├── provider.ts           # Main tree data provider implementation
├── DprTreeItem.ts        # Abstract base class for all tree items
├── DprFile.ts           # DPR file tree item (collapsible, has children)
├── DprojFile.ts         # DPROJ file tree item (opens in editor)
└── ExecutableFile.ts    # Executable file tree item (launches app)
```

## Class Hierarchy

```text
DprTreeItem (abstract)
├── DprFile           # .dpr files - can have children (dproj, exe)
├── DprojFile         # .dproj files - opens in editor when clicked
└── ExecutableFile    # .exe files - launches when clicked
```

## Key Features

- **Modular Design**: Each class is in its own file for maximum maintainability
- **Clean Imports**: Uses barrel export in `index.ts` for clean imports
- **Tree Structure**: DPR files are collapsible with DPROJ and executable as children
- **Config Caching**: Saves/loads file lists to `.vscode/delphi-utils-dpr-list.json`
- **File Watchers**: Automatically refreshes when .dpr/.dproj files change
- **Exclude Patterns**: Configurable via `delphi-utils.dprExplorer.excludePatterns`
- **Case-Insensitive**: Works with .DPR, .dpr, .DPROJ, .dproj, etc.
- **XML Parsing**: Parses .dproj files to find executable paths
- **Direct Launch**: Click executable items to launch applications

## Usage

The module exports all classes through the index file:

```typescript
import { DprExplorerProvider } from './dprExplorer';
```

Individual classes can also be imported directly:

```typescript
import { DprFile, DprojFile, ExecutableFile } from './dprExplorer';
```

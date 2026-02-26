# Change Log

All notable changes to the "delphi-devkit" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [2.0.0] - 2026-02-26

### Breaking Changes

- Removed SQLite database in favor of file-based storage (`%APPDATA%\ddk\projects.ron`, `%APPDATA%\ddk\compilers.ron`).
- Previously stored workspaces and projects **will not be migrated**. You will need to re-add them.
- Compiler configurations are no longer in VS Code settings (`ddk.compiler.configurations`); they are now managed via `compilers.ron` and the `Edit Compiler Configurations` command.

### Changed

- Full architectural rewrite: extension now communicates with a bundled Rust LSP server (`ddk-server`) over stdio. All project state, compilation, formatting, and file watching run server-side.
- Repository split into `server/` (Rust crate) and `vscode_extension/` (TypeScript).
- Drag & drop project ordering is now managed server-side.
- Author name in LICENSE/NOTICE changed

### Added

- **DDK Server**: bundled `ddk-server.exe` (Rust, async tower-lsp) handles all backend logic.
- **Preset compiler configurations**: 19 built-in entries covering Delphi 2007 through Delphi 13.0 Florence.
- **Bulk compilation**: compile or recreate all projects in a workspace or group project in a single command.
- **Cancellable compilation**: cancel any active MSBuild run via `Cancel Compilation` (Ctrl+F2); uses `taskkill /F /T` to terminate the entire process tree.
- **Formatter**: format Delphi source files via `GExperts.Formatter.exe`; configuration file editable and resettable via commands.
- **Timestamps in compiler output**: every output line is prefixed with `HH:MM:SS.mmm`.
- **Diagnostics in Problems panel**: MSBuild errors, warnings, and hints are parsed and published as LSP diagnostics.
- **File watchers**: `projects.ron` and `compilers.ron` are watched for external changes; tree views update automatically.
- **New commands**: `Compile All in Workspace`, `Recreate All in Workspace`, `Compile All in Group Project`, `Recreate All in Group Project`, `Cancel Compilation`, `Set Manual Path`, `Edit Compiler Configurations`, `Edit Projects Data`, `Edit Formatter Config`, `Reset Formatter Config`.
- **Compiler output encoding**: configurable via `ddk.compiler.encoding` setting (`oem` default).

### Fixed

- Fixed the issue where the selected project didn't work when the tree was collapsed.
- Fixed the issue where removing workspaces/projects did not work.
- Fixed the issue where Discover File Paths did not do anything.

## [1.1.0] - 2025-08-31

- Fixed the issue where the compiler's diagnostic output was always mapped as information.
- Added error code to diagnostics.
- Added support for hyperlinks in compiler output channel, enabling error/hint/warning codes to link directly to the Embarcadero documentation, and file paths to resolve.

## [1.0.0] - 2025-08-30

- Removed Project Discovery in favor of a more streamlined approach.
- Added File Explorer Icons for better visual identification of Delphi files.
- Split Delphi Projects into 2 separate views:
    - "Self-Defined Workspaces" for user-defined projects. (customizable)
    - "Loaded Group Project" for projects loaded from a .groupproj file. (readonly)
- Compiler picker is now only relevant for "Loaded Group Project" projects, so it has been clarified.
- Completely reworked backend database (you can delete old .cachedb files).
> Note: Old logic used to automatically discover projects and created a .cachedb file for each workspace (VS Code Workspace Folders + Git Status Hashed). That's why you likely can find multiple .cachedb files in the extension storage folder.

### Internal Database

- The internal database now has a root element called Configuration. This can be imported/exported as JSON using commands:
    - Export Configuration
    - Import Configuration

### Workspaces

- Added Self-Defined Workspaces:
    - Workspaces are user-defined tree items that can contain projects.
    - They have a predefined compiler assigned by the user, so all projects within the workspace will use that compiler.
    - You can create multiple workspaces and move them around the tree view as you like.
    - Dragging and dropping projects inside the tree view will move them inside or between workspaces.
    - Dragging and dropping projects from the Loaded Group Project view to a Self-Defined Workspace will copy the project to that workspace.
    - Project files are more clearly shown when missing (e.g. due to git branch variations)
- Added commands to manage Self-Defined Workspaces:
    - Add Workspace
    - Rename Workspace
    - Remove Workspace
    - Add Project
    - Remove Project
    - Discover File Paths (Reinstate the file paths in the project's database entry)

## [Unreleased]

- Initial release

# Bug Roadmap

- [x] Fixing Diagnostics to show correct types of issues.
- [x] Selected Project doesnt work when the tree is collapsed.
- [x] Removing Workspaces/Projects does not work.

# Feature Roadmap

- [x] Linking the compiler output to files.
- [x] Add timestamps to compiler output lines.
- [x] Delphi Formatter
- [x] Support for compiling / recreating all projects in a workspace / group project.
- [ ] Support for commandline execution of unit tests (DUnit).
- [ ] Integrate Delphi Language Server with background compiler. For now, you can use the [OmniPascal extension](https://marketplace.visualstudio.com/items?itemName=Wosi.omnipascal).
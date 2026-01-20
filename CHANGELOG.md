# Change Log

All notable changes to the "delphi-devkit" extension will be documented in this file.

Check [Keep a Changelog](http://keepachangelog.com/) for recommendations on how to structure this file.

## [2.0.0] - 2026-01-XX

### Breaking Changes

- Removed sqlite database in favor of direct file system storage.
- This will remove all previously stored workspaces and projects. You will need to re-add them.

### Changes

- Major overhaul of the whole extension.
- Added DDK Server
- Moved Projects logic to the server.
- Moved compilation logic to the server.

### Added

- Compiler configurations are more detailed and show compiler version details.
- More default compiler configurations have been added (All between Delphi 2007 and Delphi 13)
- Added formatter support.
- Added timestamps to compiler output lines.
- Added support for compiling / recreating all projects in a workspace / group project.

### Fixed

- Fixed the issue where the selected project didn't work when the tree was collapsed.
- Fixed the issue where removing workspaces/projects did not work.

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
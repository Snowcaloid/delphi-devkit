export const enum DelphiProjectTreeItemType {
  Project,
  DprojFile,
  DpkFile,
  DprFile,
  ExecutableFile,
  IniFile,
  ConfigurationGroup,
  PlatformGroup,
  ConfigurationItem,
  PlatformItem
}

export const enum WorkspaceViewMode {
  GroupProject,
  Discovery,
  Empty
}

export const enum ProjectFileStatus {
  Normal,
  Selected,
  Missing
}

export const enum ProjectType {
  Application,
  Package
}

export interface Feature {
  initialize(): Promise<void>;
}

export type Option<T> = T | null | undefined;

export type Coroutine<T, A extends any[] = []> = (...args: A) => Promise<T>;
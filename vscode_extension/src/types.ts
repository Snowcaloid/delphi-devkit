import { Entities } from './projects/entities';

export const enum DelphiProjectTreeItemType {
  Project,
  DprojFile,
  DpkFile,
  DprFile,
  ExecutableFile,
  IniFile
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

export namespace ExtensionDataExport {
  export enum Version {
    V1_0 = 1.0,
    V2_0 = 2.0
  }
  export const CURRENT_VERSION = Math.max(...Object.values(Version).filter((v) => typeof v === 'number')) as Version;

  export class FileContent {
    constructor(
      public readonly projectsData: Entities.ProjectsData,
      public readonly compilers: Entities.CompilerConfigurations,
      public readonly version: number = CURRENT_VERSION
    ) {}
  }
}

export type Option<T> = T | null | undefined;

export type Coroutine<T, A extends any[] = []> = (...args: A) => Promise<T>;

  export namespace Projects {
    export namespace Config {
      export const Key = 'delphi-devkit.projects';
      export function full(element: string): string {
        return `${Key}.${element}`;
      }
      export namespace Discovery {
        const NS = 'discovery';
        export const Enable                = `${NS}.enable`;
        export const UseFileSystemWatchers = `${NS}.useFileSystemWatchers`;
        export const ProjectPaths          = `${NS}.projectPaths`;
        export const ExcludePatterns       = `${NS}.excludePatterns`;
      }
      export const GitCheckoutDelay = 'gitCheckoutDelay';
      export const SortProjects     = 'sortProjects';
      export namespace Compiler {
        export const NS                   = 'compiler';
        export const Configurations       = `${NS}.configurations`;
        export const CurrentConfiguration = `${NS}.currentConfiguration`;

        export const Value_DefaultConfiguration = 'Delphi 12';
      }
    }
    export namespace Command {
      export const Refresh                     = `${Projects.Config.Key}.refresh`;
      export const LaunchExecutable            = `${Projects.Config.Key}.launchExecutable`;
      export const Compile                     = `${Projects.Config.Key}.compile`;
      export const Recreate                    = `${Projects.Config.Key}.recreate`;
      export const ShowInExplorer              = `${Projects.Config.Key}.showInExplorer`;
      export const OpenInFileExplorer          = `${Projects.Config.Key}.openInFileExplorer`;
      export const RunExecutable               = `${Projects.Config.Key}.runExecutable`;
      export const ConfigureOrCreateIni        = `${Projects.Config.Key}.configureOrCreateIni`;
      export const PickGroupProject            = `${Projects.Config.Key}.pickGroupProject`;
      export const UnloadGroupProject          = `${Projects.Config.Key}.unloadGroupProject`;
      export const SelectCompilerConfiguration = `${Projects.Config.Key}.selectCompilerConfiguration`;
      export const SelectProject               = `${Projects.Config.Key}.selectProject`;
      export const CompileSelectedProject      = `${Projects.Config.Key}.compileSelectedProject`;
      export const RecreateSelectedProject     = `${Projects.Config.Key}.recreateSelectedProject`;
      export const RunSelectedProject          = `${Projects.Config.Key}.runSelectedProject`;
    }
    
  export namespace Variables {
    export const IsGroupProjectView = 'projects.isGroupProjectView';
  }

  export namespace Context {
    export const IsGroupProjectView         = 'delphiDevkit:isGroupProjectView';
    export const IsProjectSelected          = 'delphiDevKit:isProjectSelected';
    export const DoesSelectedProjectHaveExe = 'delphiDevKit:doesSelectedProjectHaveExe';
  }

  export namespace View {
    export const Main = 'delphiProjects';
  }
  
  export namespace Scheme {
    export const Default  = 'delphi-devkit';
    export const Selected = `${Default}.selected`;
  }
}

export namespace DFM {
  export enum Commands {
    SwapToDfmPas = 'delphi-devkit.dfm.swapToDfmPas'
  }
}

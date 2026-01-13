import { commands, ExtensionContext, window } from 'vscode';
import { ProjectsFeature } from './projects/feature';
import { DfmFeature } from './dfm/feature';
import { Entities } from './projects/entities';
import { GeneralCommands } from './commands';
import { DDK_Client } from './client';
import { Option } from './types';
import { randomUUID, UUID } from 'crypto';

/**
 * Runtime class to manage workspace state and global variables.
 *
 * Properties must be synchronously accessible.
 */
export abstract class Runtime {
  public static projects: ProjectsFeature;
  public static dfm: DfmFeature;
  private static _projectsData?: Entities.ProjectsData;
  private static _compilerConfigurations?: Entities.CompilerConfigurations;
  private static _events: string[] = [];
  public static extension: ExtensionContext;
  public static client: DDK_Client;

  static async initialize(context: ExtensionContext) {
    this.extension = context;
    this.client = new DDK_Client();
    await this.client.initialize();
    this.projects = new ProjectsFeature();
    await this.projects.initialize();
    this.dfm = new DfmFeature();
    await this.dfm.initialize();
    context.subscriptions.push(
      ...GeneralCommands.registers
    );
  }

  public static get projectsData(): Option<Entities.ProjectsData> {
    return this._projectsData;
  }

  public static async getProjectsData(): Promise<Entities.ProjectsData> {
    let data = this._projectsData;
    let counter = 10;
    while (!data && counter > 0) {
      counter--;
      await new Promise((resolve) => setTimeout(resolve, 300));
      data = this._projectsData;
    }
    return data!;
  }

  public static set projectsData(value: Entities.ProjectsData) {
    this._projectsData = value;
  }

  public static get compilerConfigurations(): Option<Entities.CompilerConfigurations> {
    return this._compilerConfigurations;
  }

  public static async getCompilerConfigurations(): Promise<Entities.CompilerConfigurations> {
    let data = this._compilerConfigurations;
    while (!data) {
      await new Promise((resolve) => setTimeout(resolve, 300));
      data = this._compilerConfigurations;
    }
    return data;
  }

  public static set compilerConfigurations(value: Entities.CompilerConfigurations) {
    this._compilerConfigurations = value;
  }

  public static setContext(name: string, value: any): Thenable<void> {
    return commands.executeCommand('setContext', name, value);
  }

  public static addEvent(timeout: number = 5000): UUID {
    const id = randomUUID();
    this._events.push(id);
    setTimeout(() => {
      this.finishEvent(id);
      window.showErrorMessage(`Server Operation timed out (event id: ${id})`);
    }, timeout);
    return id;
  }

  public static finishEvent(id: string): void {
    this._events = this._events.filter((it) => it !== id);
  }

  public static async waitForEvent(id: string): Promise<void> {
    while (this._events.includes(id)) await new Promise((resolve) => setTimeout(resolve, 300));
  }
}

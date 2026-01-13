import {
    LanguageClient, LanguageClientOptions, ServerOptions, TransportKind
} from 'vscode-languageclient/node';
import { window } from 'vscode';
import * as path from 'path';
import { Runtime } from './runtime';
import { Entities } from './projects/entities';
import { UUID } from 'crypto';

export interface Change {
    type: 'NewProject' |'AddProject' | 'RemoveProject' | 'MoveProject' | 'RefreshProject' | 'UpdateProject' |'SelectProject' |
          'AddWorkspace' | 'RemoveWorkspace' | 'MoveWorkspace' | 'UpdateWorkspace' |
          'AddCompiler' | 'RemoveCompiler' | 'UpdateCompiler' | 'SetGroupProject' | 'RemoveGroupProject' | 'SetGroupProjectCompiler',
    [key: string]: any;
}

export interface Changes {
    changes: Change[];
    event_id: UUID;
}

export function newChanges(changes: Change[], timeout: number = 5000): Changes {
    const id = Runtime.addEvent(timeout);
    return { changes: changes, event_id: id };
}

export class DDK_Client {
    private client: LanguageClient;

    public async initialize(): Promise<void> {
        const serverPath = path.join(Runtime.extension.extensionPath, 'dist', 'ddk_server.exe');
        const serverOptions: ServerOptions = {
            run: { command: serverPath, transport: TransportKind.stdio },
            debug: { command: serverPath, transport: TransportKind.stdio }
        };
        const clientOptions: LanguageClientOptions = {};
        clientOptions.outputChannelName = 'DDK Server';
        this.client = new LanguageClient(
            'ddk_server',
            'DDK Server',
            serverOptions,
            clientOptions
        );
        this.client.onNotification(
            'notifications/projects/update',
            async (it: { projectsData: Entities.ProjectsData }) => {
                Runtime.projectsData = it.projectsData;
                await Runtime.projects.workspacesTreeView.refresh();
                await Runtime.projects.groupProjectTreeView.refresh();
            }
        );
        this.client.onNotification(
            'notifications/compilers/update',
            async (it: { compilers: Entities.CompilerConfigurations }) => {
                Runtime.compilerConfigurations = it.compilers;
                await Runtime.projects.compilerStatusBarItem.updateDisplay();
            }
        );
        this.client.onNotification(
            'notifications/error',
            async (it: { message: string, event_id?: string }) => {
                if (it.event_id) Runtime.finishEvent(it.event_id);
                window.showErrorMessage(`DDK Server Error: ${it.message}`);
            }
        );
        this.client.onNotification(
            'notifications/event/done',
            async (it: { event_id: string }) => {
                Runtime.finishEvent(it.event_id);
            }
        );
        await this.client.start();
    }

    public async projectsDataOverride(data: Entities.ProjectsData): Promise<void> {
        const id = Runtime.addEvent();
        Runtime.projectsData = data;
        await this.client.sendNotification('workspace/didChangeConfiguration', {
            settings: {
                projectsData: data,
                event_id: id
            }
        });
        await Runtime.waitForEvent(id);
    }

    public async compilersOverride(data: Entities.CompilerConfigurations): Promise<void> {
        const id = Runtime.addEvent();
        Runtime.compilerConfigurations = data;
        await this.client.sendNotification('workspace/didChangeConfiguration', {
            settings: {
                compilerConfigurations: data,
                event_id: id
            }
        });
        await Runtime.waitForEvent(id);
    }

    public async applyChanges(changesArray: Change[]): Promise<void> {
        const changes = newChanges(changesArray);
        await this.client.sendNotification('workspace/didChangeConfiguration', {
            settings: {
                changes: changes
            }
        });
        await Runtime.waitForEvent(changes.event_id);
    }
}
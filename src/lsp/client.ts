import { workspace } from 'vscode';
import { LanguageClient, LanguageClientOptions, ServerOptions, TransportKind } from 'vscode-languageclient/node';
import { Runtime } from '../runtime';
import { createServer } from 'net';

export class DelphiLanguageClient {
  private client: LanguageClient | undefined;
  private serverPort?: number;

  async start(): Promise<DelphiLanguageClient> {
    this.serverPort = await this.findFreePort();

    const serverOptions: ServerOptions = {
      run: {
        command: Runtime.extension.asAbsolutePath('server/dlsp.exe'),
        args: [`--port=${this.serverPort}`],
        transport: {
          kind: TransportKind.socket,
          port: this.serverPort
        }
      },
      debug: {
        command: Runtime.extension.asAbsolutePath('server/dlsp.exe'),
        args: [`--port=${this.serverPort}`],
        transport: {
          kind: TransportKind.socket,
          port: this.serverPort
        }
      }
    };

    const clientOptions: LanguageClientOptions = {
      documentSelector: [{ scheme: 'file', language: 'delphi' }],
      synchronize: {
        fileEvents: workspace.createFileSystemWatcher('**/*.{pas,dfm}')
      }
    };

    this.client = new LanguageClient(
      'delphi-devkit.lsp',
      'Delphi Language Server',
      serverOptions,
      clientOptions
    );

    await this.client.start();

    Runtime.extension.subscriptions.push({
      dispose: () => this.stop()
    });
    return this;
  }

  async stop() {
    if (this.client) {
      await this.client.stop();
      this.client = undefined;
    }
  }

  private async findFreePort(): Promise<number> {
    return new Promise((resolve, reject) => {
      const server = createServer();
      server.listen(0, () => {
        const address = server.address();
        server.close(() => {
          if (address && typeof address === 'object') {
            resolve(address.port);
          } else {
            reject(new Error('Could not find free port'));
          }
        });
      });
    });
  }
}
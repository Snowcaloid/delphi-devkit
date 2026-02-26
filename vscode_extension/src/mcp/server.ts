import * as http from 'http';
import { CancellationToken, Disposable, lm, McpHttpServerDefinition, Uri } from 'vscode';
import type { McpServerDefinitionProvider } from 'vscode';
import { Feature } from '../types';
import { Runtime } from '../runtime';
import { CompilerProgressParams } from '../client';

interface JsonRpcRequest {
  jsonrpc: '2.0';
  id: string | number | null;
  method: string;
  params?: unknown;
}

interface JsonRpcNotification {
  jsonrpc: '2.0';
  method: string;
  params?: unknown;
}

interface JsonRpcResponse {
  jsonrpc: '2.0';
  id: string | number | null;
  result?: unknown;
  error?: { code: number; message: string; data?: unknown };
}

// ---------------------------------------------------------------------------
// Tool definitions
// ---------------------------------------------------------------------------

interface McpToolDefinition {
  name: string;
  description: string;
  inputSchema: Record<string, unknown>;
}

interface McpTool {
  definition: McpToolDefinition;
  callback(args: ToolArgs): Promise<ToolResult> | ToolResult;
}

type ToolArgs = Record<string, unknown>;
type ToolResult = { content: Array<{ type: 'text'; text: string }> };

export class McpServerFeature implements Feature, McpServerDefinitionProvider {
  private server!: http.Server;
  private uri!: Uri;

  private readonly tools: Record<string, McpTool> = {
    delphi_get_environment_info: {
      definition: {
        name: 'delphi_get_environment_info',
        description:
          'Returns the currently active Delphi project and its associated compiler configuration. ' +
          'If no project is active, returns only the group project compiler configuration (if any).',
        inputSchema: { type: 'object', properties: {}, required: [] }
      },
      callback: () => {
        const project = Runtime.activeProject;
        if (!project) {
          const compiler = Runtime.groupProjectsCompiler;
          return {
            content: [{
              type: 'text' as const,
              text: compiler
                ? `No active project.\n\nGroup project compiler:\n${JSON.stringify(compiler, null, 2)}`
                : 'No active project and no group project compiler configured.'
            }]
          };
        }

        const links = Runtime.getLinksOfProject(project);
        const compilers: Record<string, unknown> = {};
        for (const link of links) {
          const ws = Runtime.getWorkspaceOfLink(link);
          if (ws) {
            const compiler = Runtime.getCompilerOfWorkspace(ws);
            if (compiler) compilers[ws.name] = compiler;
          } else {
            const compiler = Runtime.groupProjectsCompiler;
            if (compiler) compilers['group_project'] = compiler;
          }
        }

        return { content: [{ type: 'text' as const, text: JSON.stringify({ project, compilers }, null, 2) }] };
      }
    },

    delphi_compile_selected_project: {
      definition: {
        name: 'delphi_compile_selected_project',
        description: 'Compiles the currently selected/active Delphi project.',
        inputSchema: { type: 'object', properties: {}, required: [] }
      },
      callback: async () => {
        const project = Runtime.activeProject;
        if (!project) return { content: [{ type: 'text' as const, text: 'No active project selected.' }] };

        const links = Runtime.getLinksOfProject(project);
        if (!links.length) return { content: [{ type: 'text' as const, text: `Project "${project.name}" has no compiled links.` }] };

        const outputLines: string[] = [];
        const listener = (params: CompilerProgressParams) => {
          switch (params.kind) {
            case 'Start':
            case 'SingleProjectStarted':
            case 'Completed':
            case 'SingleProjectCompleted':
              outputLines.push(...params.lines);
              break;
            case 'Stdout':
            case 'Stderr':
              outputLines.push(params.line);
              break;
          }
        };

        Runtime.client.addCompilerProgressListener(listener);
        let success = false;
        try {
          success = await Runtime.compileProjectLink(links[0]);
        } finally {
          Runtime.client.removeCompilerProgressListener(listener);
        }

        const summary = success
          ? `Project "${project.name}" compiled successfully.`
          : `Compilation of "${project.name}" finished with errors.`;
        const output = outputLines.length ? `\n\nCompiler output:\n${outputLines.join('\n')}` : '';
        return { content: [{ type: 'text' as const, text: summary + output }] };
      }
    }
  };

  async initialize(): Promise<void> {
    this.server = await new Promise<http.Server>((resolve, reject) => {
      const server = http.createServer(this.handleRequest.bind(this));
      server.once('error', reject);
      server.listen(0, '127.0.0.1', () => resolve(server));
    });
    const addr = this.server.address() as { port: number };
    this.uri = Uri.parse(`http://127.0.0.1:${addr.port}/mcp`);
    Runtime.extension.subscriptions.push(
      lm.registerMcpServerDefinitionProvider('ddk.mcp', this),
      Disposable.from({ dispose: () => this.server.close() })
    );
  }

  provideMcpServerDefinitions(_token: CancellationToken) {
    return [new McpHttpServerDefinition('DDK - Delphi Development Kit', this.uri)];
  }

  private handleRequest(req: http.IncomingMessage, res: http.ServerResponse): void {
    if (req.method !== 'POST') {
      res.writeHead(405).end();
      return;
    }
    let body = '';
    req.on('data', (chunk) => (body += chunk));
    req.on('end', async () => {
      let parsed: JsonRpcRequest | JsonRpcNotification;
      try {
        parsed = JSON.parse(body);
      } catch {
        res.writeHead(400, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Parse error' } }));
        return;
      }
      const response = await this.dispatch(parsed);
      res.writeHead(200, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' });
      res.end(response ? JSON.stringify(response) : JSON.stringify({ jsonrpc: '2.0', id: null, result: null }));
    });
  }

  private dispatch(req: JsonRpcRequest | JsonRpcNotification): Promise<JsonRpcResponse | null> | JsonRpcResponse | null {
    const id = 'id' in req ? req.id : null;
    if (!('id' in req)) return null;

    switch (req.method) {
      case 'initialize':
        return {
          jsonrpc: '2.0', id,
          result: {
            protocolVersion: '2024-11-05',
            capabilities: { tools: {} },
            serverInfo: { name: 'ddk-mcp', version: '1.0.0' }
          }
        };

      case 'tools/list':
        return { jsonrpc: '2.0', id, result: { tools: Object.values(this.tools).map((e) => e.definition) } };

      case 'tools/call': {
        const params = req.params as { name?: string; arguments?: ToolArgs } | undefined;
        const name = params?.name ?? '';
        const args = params?.arguments ?? {};
        const entry = this.tools[name];
        const result = entry
          ? Promise.resolve(entry.callback(args))
          : Promise.resolve({ content: [{ type: 'text' as const, text: `Unknown tool: ${name}` }] });
        return result.then(
          (r) => ({ jsonrpc: '2.0' as const, id, result: r }),
          (e) => ({ jsonrpc: '2.0' as const, id, error: { code: -32603, message: String(e) } })
        );
      }

      default:
        return { jsonrpc: '2.0', id, error: { code: -32601, message: `Method not found: ${req.method}` } };
    }
  }

}


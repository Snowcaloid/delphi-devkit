import { CancellationToken, ExtensionMode, lm, McpStdioServerDefinition } from 'vscode';
import type { McpServerDefinitionProvider } from 'vscode';
import { Feature } from '../types';
import { Runtime } from '../runtime';
import { join } from 'path';
import { existsSync } from 'fs';

/**
 * Registers ddk-mcp-server (Rust STDIO binary) with VS Code's MCP infrastructure.
 *
 * VS Code spawns the binary as a child process and uses its stdin/stdout for
 * the MCP protocol. External tools such as Claude Desktop can also speak to
 * this server via STDIO – no HTTP proxy or port discovery required.
 *
 * State is shared with ddk-server through RON files on disk, so project and
 * compiler changes from the MCP server are reflected in the extension tree
 * automatically via the file watcher.
 */
export class McpServerFeature implements Feature, McpServerDefinitionProvider<McpStdioServerDefinition> {
  async initialize(): Promise<void> {
    Runtime.extension.subscriptions.push(
      lm.registerMcpServerDefinitionProvider('ddk.mcp', this),
    );
  }

  provideMcpServerDefinitions(_token: CancellationToken) {
    const serverPath = this.resolveServerPath();
    if (!existsSync(serverPath)) {
      console.warn(`[DDK] ddk-mcp-server not found at: ${serverPath}`);
      return [];
    }
    return [new McpStdioServerDefinition('DDK - Delphi Development Kit', serverPath, [])];
  }

  private resolveServerPath(): string {
    const ext = Runtime.extension;
    const isDev = ext.extensionMode !== ExtensionMode.Production;
    return isDev
      ? join(ext.extensionUri.fsPath, '..', 'target', 'debug', 'ddk-mcp-server.exe')
      : join(ext.extensionUri.fsPath, 'server', 'ddk-mcp-server.exe');
  }
}



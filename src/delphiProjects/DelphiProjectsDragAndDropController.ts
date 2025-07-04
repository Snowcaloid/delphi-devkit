import { TreeDragAndDropController, DataTransfer, DataTransferItem } from 'vscode';
import { DelphiProjectTreeItem } from './treeItems/DelphiProjectTreeItem';
import { ProjectDiscovery } from './data/projectDiscovery';
import { ProjectCacheManager } from './data/cacheManager';

export class DelphiProjectsDragAndDropController implements TreeDragAndDropController<DelphiProjectTreeItem> {
  readonly dragMimeTypes = ['application/vnd.code.tree.delphiProjects'];
  readonly dropMimeTypes = ['application/vnd.code.tree.delphiProjects'];
  private cacheManager = new ProjectCacheManager();
  private customOrder: string[] | undefined;

  async handleDrag(source: DelphiProjectTreeItem[], dataTransfer: DataTransfer): Promise<void> {
    dataTransfer.set('application/vnd.code.tree.delphiProjects', new DataTransferItem(source.map(item => this.getProjectKey(item))));
  }

  async handleDrop(target: DelphiProjectTreeItem | undefined, dataTransfer: DataTransfer): Promise<void> {
    const raw = dataTransfer.get('application/vnd.code.tree.delphiProjects');
    if (!raw) { return; }
    const draggedKeys: string[] = raw.value;
    if (!Array.isArray(draggedKeys) || draggedKeys.length === 0) { return; }
    let order = await this.getCurrentOrder();
    order = order.filter(key => !draggedKeys.includes(key));
    let targetKey = target ? this.getProjectKey(target) : undefined;
    let insertIndex = targetKey ? order.indexOf(targetKey) : order.length;
    order.splice(insertIndex, 0, ...draggedKeys);
    this.customOrder = order;
    await this.saveCustomOrder(order);
  }

  private getProjectKey(item: DelphiProjectTreeItem): string {
    // Use absolute path as unique key
    // @ts-ignore
    return item.dpr?.fsPath || item.dproj?.fsPath || item.dpk?.fsPath || item.executable?.fsPath || item.ini?.fsPath || item.resourceUri?.fsPath || item.label;
  }

  private async getCurrentOrder(): Promise<string[]> {
    if (this.customOrder) { return this.customOrder; }
    const configData = await this.cacheManager.loadCacheData();
    if (configData?.customOrder) { return configData.customOrder; }
    const projects = await ProjectDiscovery.getAllProjects();
    return projects.map(p => p.dpr?.fsPath || p.dproj?.fsPath || p.dpk?.fsPath || p.executable?.fsPath || p.ini?.fsPath || p.label);
  }

  private async saveCustomOrder(order: string[]): Promise<void> {
    const configData = await this.cacheManager.loadCacheData() || { lastUpdated: new Date().toISOString(), version: '1.0', defaultProjects: [] };
    configData.customOrder = order;
    await this.cacheManager.saveCacheData(configData);
  }
}

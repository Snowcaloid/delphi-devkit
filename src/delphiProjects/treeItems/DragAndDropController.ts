import { TreeDragAndDropController, DataTransfer, DataTransferItem } from 'vscode';
import { DelphiProjectTreeItem } from './DelphiProjectTreeItem';
import { ProjectDiscovery } from '../data/projectDiscovery';
import { ProjectCacheManager } from '../data/cacheManager';

export class DelphiProjectsDragAndDropController implements TreeDragAndDropController<DelphiProjectTreeItem> {
  readonly dragMimeTypes = ['application/vnd.code.tree.delphiProjects'];
  readonly dropMimeTypes = ['application/vnd.code.tree.delphiProjects'];
  private cacheManager = new ProjectCacheManager();
  private customOrder: string[] | undefined;
  public groupCustomOrder: string[] | undefined;
  private onOrderChanged?: () => void;

  constructor(onOrderChanged?: () => void) {
    this.onOrderChanged = onOrderChanged;
  }

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
    // If the target is a child, use its parent as the drop target
    let targetKey = target ? this.getProjectKey(target) : undefined;
    if (target && target.parent) {
      targetKey = this.getProjectKey(target.parent);
    }
    // Prevent dropping a project onto itself or its own children
    if (targetKey && draggedKeys.includes(targetKey)) {
      return;
    }
    let insertIndex = targetKey ? order.indexOf(targetKey) : order.length;
    order.splice(insertIndex, 0, ...draggedKeys);
    // Detect if we're in group project mode by checking the cache
    const configData = await this.cacheManager.loadCacheData();
    if (configData && configData.currentGroupProject) {
      this.groupCustomOrder = order;
    } else {
      this.customOrder = order;
      await this.saveCustomOrder(order);
    }
    if (this.onOrderChanged) {
      this.onOrderChanged();
    }
  }

  private getProjectKey(item: DelphiProjectTreeItem): string {
    // Use absolute path as unique key
    // @ts-ignore
    return item.dpr?.fsPath || item.dproj?.fsPath || item.dpk?.fsPath || item.executable?.fsPath || item.ini?.fsPath || item.resourceUri?.fsPath || item.label;
  }

  private async getCurrentOrder(): Promise<string[]> {
    // Check for group project mode
    const configData = await this.cacheManager.loadCacheData();
    if (configData && configData.currentGroupProject && this.groupCustomOrder) {
      return this.groupCustomOrder;
    }
    if (this.customOrder) { return this.customOrder; }
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

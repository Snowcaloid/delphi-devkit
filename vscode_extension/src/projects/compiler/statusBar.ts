import { window, StatusBarAlignment, StatusBarItem } from 'vscode';
import { Runtime } from '../../runtime';
import { PROJECTS } from '../../constants';

export class CompilerPicker {
  private statusBarItem: StatusBarItem;

  constructor() {
    this.statusBarItem = window.createStatusBarItem(PROJECTS.STATUS_BAR.COMPILER, StatusBarAlignment.Left, 0);
    this.statusBarItem.command = PROJECTS.COMMAND.SELECT_COMPILER;
    this.statusBarItem.tooltip = 'Select Delphi Compiler Configuration';
    this.updateDisplay();
    this.statusBarItem.show();
    Runtime.extension.subscriptions.push(this.statusBarItem);
  }

  public async updateDisplay(): Promise<void> {
    try {
      const data = await Runtime.getProjectsData();
      const currentConfigName = data.group_project?.compiler?.product_name || 'No Compiler';
      this.statusBarItem.text = `$(tools) .groupproj Compiler: ${currentConfigName}`;
    } catch (error) {
      this.statusBarItem.text = '$(tools) No .groupproj Compiler';
    }
  }
}

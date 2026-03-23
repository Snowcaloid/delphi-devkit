import { QuickPickItem, QuickPickItemKind, window } from 'vscode';
import { Runtime } from '../../runtime';
import { Entities } from '../entities';

interface ProjectPickItem extends QuickPickItem {
  project?: Entities.Project;
}

export async function pickAndSelectProject(): Promise<void> {
  const data = Runtime.projectsData;
  if (!data) {
    window.showErrorMessage('No project data available.');
    return;
  }

  const items: ProjectPickItem[] = [];

  for (const ws of data.workspaces) {
    const links = ws.project_links;
    if (!links.length) continue;

    items.push({ label: ws.name, kind: QuickPickItemKind.Separator });

    for (const link of links) {
      const project = data.projects.find((p) => p.id === link.project_id);
      if (!project) continue;
      const isActive = project.id === data.active_project_id;
      items.push({
        label: (isActive ? '$(check) ' : '$(circle-outline) ') + project.name,
        description: project.directory,
        picked: isActive,
        project
      });
    }
  }

  if (data.group_project) {
    const links = data.group_project.project_links;
    if (links.length) {
      items.push({ label: data.group_project.name, kind: QuickPickItemKind.Separator });
      for (const link of links) {
        const project = data.projects.find((p) => p.id === link.project_id);
        if (!project) continue;
        const isActive = project.id === data.active_project_id;
        items.push({
          label: (isActive ? '$(check) ' : '$(circle-outline) ') + project.name,
          description: project.directory,
          picked: isActive,
          project
        });
      }
    }
  }

  if (!items.some((i) => i.project)) {
    window.showInformationMessage('No projects available to select.');
    return;
  }

  const picked = await window.showQuickPick(items, {
    title: 'Select Active Project',
    placeHolder: 'Type to filter projects…',
    matchOnDescription: true
  });

  if (!picked?.project) return;

  await Runtime.client.applyChanges([{ type: 'SelectProject', project_id: picked.project.id }]);
}

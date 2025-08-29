import 'reflect-metadata';
import { Column, Entity, JoinColumn, ManyToOne, OneToMany, OneToOne, PrimaryGeneratedColumn } from 'typeorm';
import { SortedItem } from '../utils/lexoSorter';
import { ProjectLinkType } from '../types';
import { Runtime } from '../runtime';

export namespace Entities {
  @Entity()
  export class Configuration {
    @Column({ primary: true, type: 'int', default: 0 })
    id: number;

    @OneToMany(() => Workspace, (workspace) => workspace.configuration, {
      cascade: true,
      eager: true
    })
    workspaces: Workspace[];

    @Column({ type: 'varchar', length: 50, nullable: true })
    groupProjectsCompiler?: string | null;

    @OneToOne(() => Project, { nullable: true, eager: true })
    @JoinColumn()
    selectedProject?: Project | null;

    @OneToOne(() => GroupProject, { nullable: true, eager: true })
    @JoinColumn()
    selectedGroupProject?: GroupProject | null;
  }

  export interface ProjectOwner {
    id: number;
    name: string;
    projects: ProjectLink[];
  }

  @Entity()
  export class Workspace implements ProjectOwner, SortedItem {
    @PrimaryGeneratedColumn()
    id: number;

    @ManyToOne(() => Configuration, (configuration) => configuration.workspaces)
    configuration: Configuration;

    @Column({ type: 'varchar', length: 50 })
    name: string;

    @Column({ type: 'varchar', length: 50 })
    compiler: string;

    @OneToMany(() => WorkspaceLink, (workspaceLink) => workspaceLink.workspace, { cascade: true, eager: true })
    projects: WorkspaceLink[];

    @Column({ type: 'varchar', length: 1024 })
    sortValue: string;
  }

  @Entity()
  export class GroupProject implements ProjectOwner {
    @PrimaryGeneratedColumn()
    id: number;

    @Column({ type: 'varchar', length: 50 })
    name: string;

    @Column({ type: 'varchar', length: 255 })
    path: string;

    @OneToMany(() => GroupProjectLink, (groupProjectLink) => groupProjectLink.groupProject, { cascade: true, eager: true })
    projects: GroupProjectLink[];
  }

  @Entity()
  export class Project {
    @PrimaryGeneratedColumn()
    id: number;

    @OneToMany(() => WorkspaceLink, (workspaceLink) => workspaceLink.project)
    workspaces: WorkspaceLink[];

    @OneToMany(() => GroupProjectLink, (groupProjectProject) => groupProjectProject.project)
    groupProjects: GroupProjectLink[];

    @Column({ type: 'varchar', length: 50 })
    name: string;

    @Column({ type: 'varchar', length: 255 })
    path: string;

    @Column({ type: 'text', nullable: true })
    dproj?: string | null;

    @Column({ type: 'text', nullable: true })
    dpr?: string | null;

    @Column({ type: 'text', nullable: true })
    dpk?: string | null;

    @Column({ type: 'text', nullable: true })
    exe?: string | null;

    @Column({ type: 'text', nullable: true })
    ini?: string | null;
  }

  export interface ProjectLink extends SortedItem {
    id: number;
    project: Project;
    sortValue: string;
    linkType: ProjectLinkType;

    workspaceSafe: Workspace | undefined | null;
    groupProjectSafe: GroupProject | undefined | null;
  }

  // Join entity for WorkspaceEntity and ProjectEntity with sort value
  @Entity()
  export class WorkspaceLink implements ProjectLink {
    @PrimaryGeneratedColumn()
    id: number;

    @ManyToOne(() => Workspace, (workspace) => workspace.projects)
    workspace: Workspace;

    @ManyToOne(() => Project, (project) => project.workspaces, {
      eager: true,
      cascade: true
    })
    project: Project;

    @Column({ type: 'varchar', length: 1024 })
    sortValue: string;

    get linkType(): ProjectLinkType {
      return ProjectLinkType.Workspace;
    }

    get workspaceSafe(): Workspace | undefined | null {
      if (this.workspace) return this.workspace;

      for (const ws of Runtime.configEntity.workspaces ?? []) if (ws.projects.some((link) => link.id === this.id)) return ws;
    }

    get groupProjectSafe(): null {
      return null;
    }
  }

  @Entity()
  export class GroupProjectLink implements ProjectLink {
    @PrimaryGeneratedColumn()
    id: number;

    @ManyToOne(() => GroupProject, (groupProject) => groupProject.projects)
    groupProject: GroupProject;

    @ManyToOne(() => Project, (project) => project.groupProjects, {
      eager: true,
      cascade: true
    })
    project: Project;

    @Column({ type: 'varchar', length: 1024 })
    sortValue: string;

    get linkType(): ProjectLinkType {
      return ProjectLinkType.GroupProject;
    }

    get workspaceSafe(): null {
      return null;
    }

    get groupProjectSafe(): GroupProject | undefined {
      if (this.groupProject) return this.groupProject;

      if (Runtime.configEntity.selectedGroupProject?.projects.some((link) => link.id === this.id)) return Runtime.configEntity.selectedGroupProject;
    }
  }

  export const ALL = [Configuration, Workspace, GroupProject, Project, WorkspaceLink, GroupProjectLink];
}

import type { Changes, Change } from './types/changes';

export declare function addChange(change: Change, deploy_envs?: string[], cwd?: string): boolean

export declare function initChanges(cwd?: string | undefined | null): Changes

export declare function removeChange(branch: string, cwd?: string): boolean


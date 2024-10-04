import type { Result } from './types/general';

import type { Changes, Change } from './types/changes';

export declare function addChange(change: Change, deploy_envs?: string[], cwd?: string): Result<boolean>

export declare function getChanges(cwd?: string): Result<Changes>

export declare function getChangesByBranch(branch: string, cwd?: string): Result<{deploy: string[]; pkgs: Changes[]}>

export declare function initChanges(cwd?: string | undefined | null): Result<Changes>

export declare function removeChange(branch: string, cwd?: string): boolean


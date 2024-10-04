import type { PackageManager } from './types/manager';

import type { WorkspaceConfig } from './types/config';

import type { Result } from './types/general';

import type { Changes, Change } from './types/changes';

export declare function addChange(change: Change, deploy_envs?: string[], cwd?: string): Result<boolean>

export declare function changeExists(branch: string, package: string, cwd?: string): boolean

export declare function detectPackageManager(cwd: string): Result<PackageManager>

export declare function getChanges(cwd?: string): Result<Changes>

export declare function getChangesByBranch(branch: string, cwd?: string): Result<{deploy: string[]; pkgs: Changes[]}|null>

export declare function getChangesByPackage(package: string, branch: string, cwd?: string): Result<Change|null>

export declare function getConfig(cwd?: string): Result<WorkspaceConfig>

export declare function initChanges(cwd?: string | undefined | null): Result<Changes>

export declare function removeChange(branch: string, cwd?: string): boolean


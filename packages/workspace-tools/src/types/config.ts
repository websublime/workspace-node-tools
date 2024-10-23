import type { PackageManager } from './enums'

export type CliffBump = 'major' | 'minor' | 'patch'

export interface CliffRemoteConfig {
  github: CliffRemote
  gitlab: CliffRemote
  bitbucket: CliffRemote
  gitea: CliffRemote
}

export interface CliffRemote {
  owner: string
  repo: string
  token?: string
  isCustom: boolean
}

export interface CliffConfig {
  changelog: CliffChangelogConfig
  git: CliffGitConfig
  remote: CliffRemoteConfig
  bump: CliffBump
}

export interface CliffLinkParser {
  pattern: string
  href: string
  text?: string
}

export interface CliffCommitParser {
  sha?: string
  message?: string
  body?: string
  footer?: string
  group?: string
  defaultScope?: string
  scope?: string
  skip?: boolean
  field?: string
  pattern?: string
}

export interface CliffTextProcessor {
  pattern: string
  replace?: string
  replaceCommand?: string
}

export interface CliffGitConfig {
  conventionCommits?: boolean
  filterUnconventional?: boolean
  splitCommits?: boolean
  commitPreprocessors?: CliffTextProcessor[]
  commitParsers?: CliffCommitParser[]
  protectBreakingCommits?: boolean
  linkParsers?: CliffLinkParser[]
  filterCommits?: boolean
  tagPattern?: string
  skipTags?: string
  ignoreTags?: string
  countTags?: string
  useBranchTags?: boolean
  topoOrder?: boolean
  sortCommits?: string
  limitCommits?: number
}

export interface CliffChangelogConfig {
  header?: string
  body?: string
  footer?: string
  trim?: boolean
  renderAlways?: boolean
  postprocessors: CliffTextProcessor[]
  output?: string
}

export interface ToolsConfig {
  tools: ToolsConfigGroup
}

export interface ToolsConfigGroup {
  bumpSync: boolean
}

export interface WorkspaceConfig {
  workspaceRoot: string
  toolsConfig: ToolsConfig
  packageManager: PackageManager
  cliffConfig: CliffConfig
  changesConfig: Record<string, string>
}

export type BumpType = 'major' | 'minor' | 'patch' | 'snapshot' | 'beta' | 'alpha' | 'rc'

export interface Change {
  package: string
  releaseAs: BumpType
}

export interface Changes {
  [key: string]: {
    deploy: string[]
    pkgs: Change[]
  }
}

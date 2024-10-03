export type BumpType = 'major' | 'minor' | 'patch' | 'snapshot'

export interface Change {
  package: string
  release_as: BumpType
}

export interface Changes {
  [key: string]: {
    deploy: string[]
    packages: Change[]
  }
}

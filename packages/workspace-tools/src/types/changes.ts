export interface Change {
  depoy: string[]
  package: string
  release_as: string
}

export interface Changes {
  [key: string]: Change[]
}

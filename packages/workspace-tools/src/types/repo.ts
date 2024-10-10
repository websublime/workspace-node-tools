export interface RepositoryCommit {
  hash: string
  authorName: string
  authorEmail: string
  authorDate: string
  message: string
}

export interface RepositoryRemoteTags {
  hash: string
  tag: string
}

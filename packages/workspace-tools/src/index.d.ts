export declare interface Change {
    depoy: string[]
    package: string
    release_as: string
}

export declare interface Changes {
    [key: string]: Change[]
}

export declare function initChanges(cwd?: string | undefined | null): Changes

export { }

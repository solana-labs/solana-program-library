export type DatabaseConfig = {
    tableDir: string,
    reset: boolean,
}

export const DEFAULT_DB_FILE_NAME = "concurrent_merkle_tree.db";
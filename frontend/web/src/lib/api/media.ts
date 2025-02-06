import { Path } from "./path";
import { BaseClient } from "./types";

interface SFileRaw {
    id: number,
    media_id?: number,
    is_dir: boolean,
    full_path: string,
    created_at: string,
    modified_at: string,
    // Either the name of the directory or the file
    top_level_name: string
}

export interface SFile {
    id: number,
    referencesMediaId?: number,
    isDir: boolean,
    fullPath: Path,
    createdAt: Date,
    modifiedAt: Date,
    // Either the name of the directory or the file
    topLevelName: string
}

interface CacheEntry {
    mediaId: number;
    blob: Blob;
    cachedAt: Date;
}

class MediaCache {
    private db: IDBDatabase | null = null;
    private readonly DB_NAME = "MediaCache";
    private readonly STORE_NAME = "files";
    private readonly DB_VERSION = 1;
    public initialized = false;
    private CACHE_DURATION = 1000 * 60 * 60; // 1 hour
    private MAX_CACHED_FILE_SIZE = 200 * 1_000_000 // 200 mb

    // TODO! remove all expired entries on init.
    // otherwise too many files will stay cached forever.
    async init(): Promise<boolean> {
        return new Promise((resolve, reject) => {
            if (this.initialized && this.db) {
                resolve(true);
                return;
            }

            const request = indexedDB.open(this.DB_NAME, this.DB_VERSION);

            request.onerror = () => {
                console.error("Failed to open database");
                reject(new Error("Failed to open database"));
            };

            request.onsuccess = (event) => {
                this.db = (event.target as IDBOpenDBRequest).result;
                this.initialized = true;
                resolve(true);
            };

            request.onupgradeneeded = (event) => {
                const db = (event.target as IDBOpenDBRequest).result;
                
                if (!db.objectStoreNames.contains(this.STORE_NAME)) {
                    const store = db.createObjectStore(this.STORE_NAME, { keyPath: "mediaId" });
                    store.createIndex("createdAt", "createdAt", { unique: false });
                }
            };
        });
    }

    private async getFromDb(mediaId: number): Promise<CacheEntry | null> {
        if (!this.db) {
            throw new Error("Database not initialized");
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.STORE_NAME], "readonly");
            const store = transaction.objectStore(this.STORE_NAME);
            const request = store.get(mediaId);

            request.onsuccess = () => {
                resolve(request.result || null);
            };

            request.onerror = () => {
                console.error("Error fetching from cache");
                reject(new Error("Error fetching from cache"));
            };
        });
    }

    private async saveToDb(entry: CacheEntry): Promise<void> {
        if (!this.db) {
            throw new Error("Database not initialized");
        }

        return new Promise((resolve, reject) => {
            const transaction = this.db!.transaction([this.STORE_NAME], "readwrite");
            const store = transaction.objectStore(this.STORE_NAME);
            const request = store.put(entry);

            request.onsuccess = () => {
                resolve();
            };

            request.onerror = () => {
                console.error("Error saving to cache");
                reject(new Error("Error saving to cache"));
            };
        });
    }
  
    async get(file: SFile): Promise<Blob | null> {
        if (!this.initialized)
            throw new Error("Using uninitialized cache.");
        
        if (!file.referencesMediaId)
            return null;
        
        try {
            const entry = await this.getFromDb(file.referencesMediaId);

            // Return null if no entry found, if entry is expired,
            // or if the file was "modified" more recently than the cache.
            if (!entry 
                || Date.now() - entry.cachedAt.getTime() > this.CACHE_DURATION
                || file.modifiedAt > entry.cachedAt) {
                    console.log(file.modifiedAt.toString());
                    console.log(entry?.cachedAt.toString());
                return null;
            }

            return entry.blob;
        } catch (error) {
            console.error('Error retrieving from cache:', error);
            return null;
        }
    }
    
    async put(mediaId: number, blob: Blob) {
        if (!this.initialized)
            throw new Error("Using uninitialized cache.");

        if (blob.size > this.MAX_CACHED_FILE_SIZE) {
            console.log("File too big to cache, too bad.");
            return;
        }

        const entry: CacheEntry = {
            mediaId,
            blob,
            cachedAt: new Date(Date.now())
        };

        try {
            await this.saveToDb(entry);
            console.log("saved!");
        } catch (e) {
            console.log(`Failed to save blob to cache: ${e}`);
        }
    }
}

function sfile_from_raw(raw: SFileRaw): SFile {
    return {
        id: raw.id,
        isDir: raw.is_dir,
        fullPath: new Path(raw.full_path),
        createdAt: new Date(raw.created_at),
        modifiedAt: new Date(raw.modified_at),
        // Either the name of the directory or the file
        topLevelName: raw.top_level_name,
        referencesMediaId: raw.media_id
    };
}

export class MediaApi extends BaseClient {
    private cache = new MediaCache();

    async initCache(): Promise<boolean> {
        return this.cache.init();
    }

    async listDir(dir: Path): Promise<SFile[] | null> {
        const _dir = dir.asDir();
        const raw = await this.request<SFileRaw[]>(
            `/files/${_dir.toString()}`,
            {
                method: "GET"
            }
        );
        
        if (!raw)
            return null;

        return raw.map(sfile_from_raw);
    }

    async getMedia(file: SFile, useCache: boolean = true): Promise<Blob | null> {
        const _file = file.fullPath.asFile();
        if (!file.referencesMediaId)
            return null;

        if (useCache) {
            if (!this.cache.initialized) {
                await this.cache.init();
                if (!this.cache.initialized) {
                    throw new Error("Could not init the cache.");
                }
            }

            const cached = await this.cache.get(file);
            if (cached) {
                return cached;
            }
            console.log("found nothing in cache... :(");
        }

        const blob = await this.requestBytes(
            `/files/${_file.toString()}`,
            {
                method: "GET"
            }
        );

        if (!blob)
            return null;

        if (useCache) {
            await this.cache.put(file.referencesMediaId, blob);
        }

        return blob;
    }

    async uploadFile(dir: Path, file: File): Promise<SFile | null> {
        const uploadPath = dir.asDir();
        const formData = new FormData();
        formData.append(file.name, file);
        const raw = await this.request<SFileRaw[]>(
            `/files/${uploadPath}`, 
            {
                method: "POST",
                body: formData,
            }
        );

        if (!raw)
            return null;

        return sfile_from_raw(raw[0]);
    }

    async mkDirs(dir: Path): Promise<SFile[] | null> {
        const uploadPath = dir.asDir();
        const raw = await this.request<SFileRaw[]>(
            `/files/${uploadPath}`, 
            {
                method: "POST",
            }
        );

        if (!raw)
            return null;

        return raw.map(sfile_from_raw);
    }
}
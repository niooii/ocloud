import { Path } from "./path";
import { BaseClient } from "./types";
import { SFile } from "./types";

interface SFileRaw {
    id: number,
    is_dir: boolean,
    full_path: string,
    created_at: string,
    modified_at: string,
    // Either the name of the directory or the file
    top_level_name: string
}

class MediaCache {
    private db: IDBDatabase | null = null;
    private readonly DB_NAME = 'MediaCache';
    private readonly STORE_NAME = 'files';
    private readonly DB_VERSION = 1;
    public initialized = false;
    private CACHE_DURATION = 1000 * 60 * 60; // 1 hour

    async init(): Promise<boolean> {
        // ??? just read the docs bruh
        // const request = indexedDB.open(this.DB_NAME, this.DB_VERSION);
      
        // request.onupgradeneeded = (event) => {
        //     const db = (event.target as IDBOpenDBRequest).result;
            
        //     if (!db.objectStoreNames.contains(this.STORE_NAME)) {
        //         const store = db.createObjectStore(this.STORE_NAME, { keyPath: 'id' });
                
        //         store.createIndex("sfile", "sfile");
        //     }
        // };
  
        // request.onsuccess = (event) => {
        //     this.db = (event.target as IDBOpenDBRequest).result;
        //     return true;
        // };
  
        // request.onerror = (event) => {
        //     return false;
        // };
    }
  
    async get(fileId: number): Promise<Blob | null> {
        // const entry = await this.getFromDB(fileId);
        // return null if out of date        
        // return entry.blob;
    }
  
    async set(fileId: number, blob: Blob): Promise<void> {
        // await this.saveToDB(entry);
    }
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

        return raw.map((raw): SFile => {
            return {
                id: raw.id,
                isDir: raw.is_dir,
                fullPath: new Path(raw.full_path),
                createdAt: new Date(raw.created_at),
                modifiedAt: new Date(raw.modified_at),
                // Either the name of the directory or the file
                topLevelName: raw.top_level_name
            };
        });
    }

    async getMedia(file: Path): Promise<Blob | null> {
        const _file = file.asFile();
        const blob = await this.requestBytes(
            `/files/${_file.toString()}`,
            {
                method: "GET"
            }
        );

        if (!blob)
            return null;

        return blob;
    }

    async uploadFile(dir: Path, file: File): Promise<null> {
        const uploadPath = dir.joinStr(file.name);
        const result = await this.requestString(
            `/media/${uploadPath}`, 
            {
                method: "POST",
                // how to send multipart
            }
        );

        return null;
    }
}
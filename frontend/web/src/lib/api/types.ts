import { Path } from "./path";

interface SFileRaw {
    id: number,
    is_dir: boolean,
    full_path: string,
    created_at: string,
    modified_at: string,
    // Either the name of the directory or the file
    top_level_name: string
}

export interface SFile {
    id: number,
    isDir: boolean,
    fullPath: Path,
    createdAt: Date,
    modifiedAt: Date,
    // Either the name of the directory or the file
    topLevelName: string
}

export class BaseClient {
    protected serverUrl: string;
    protected headers: Record<string, string>;
  
    constructor(serverUrl: string) {
        this.serverUrl = serverUrl;
        this.headers = {};
    }
  
    protected getHeaders(): Record<string, string> {
        const token = localStorage.getItem('OCLOUD_AUTH');
            if (token) {
                return {
                    ...this.headers,
                    "Authorization": `Bearer ${token}`,
                };
            }
        return this.headers;
    }
  
    protected async request<T>(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<T> {
        const response = await fetch(`${this.serverUrl}${endpoint}`, {
            ...options,
            headers: {
                ...this.getHeaders(),
                ...options.headers,
            },
        });
    
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.message || 'API request failed');
        }
    
        return response.json();
    }
  
    protected async requestBytes(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<Blob> {
        const response = await fetch(`${this.serverUrl}${endpoint}`, {
            ...options,
            headers: {
                ...this.getHeaders(),
                ...options.headers,
            },
        });
    
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.message || 'API request failed');
        }
    
        return response.blob();
    }
  
    protected async requestString(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<string> {
        const response = await fetch(`${this.serverUrl}${endpoint}`, {
            ...options,
            headers: {
                ...this.getHeaders(),
                ...options.headers,
            },
        });
    
        if (!response.ok) {
            const error = await response.json();
            throw new Error(error.message || 'API request failed');
        }
    
        return response.text();
    }
}
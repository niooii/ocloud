import { Path } from "./path";

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

    private _fetch(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<Response> {
        return fetch(`${this.serverUrl}${endpoint}`, {
            ...options,
            headers: {
                ...this.getHeaders(),
                ...options.headers,
            },
        });
    }
  
    protected async request<T>(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<T | null> {
        try {
            const response = await this._fetch(endpoint, options);
        
            if (!response.ok) {
                const error = await response.json();
                console.log(`Request failed: ${error}`);
                return null;
            }

            return response.json();
        } catch (e) {
            console.log(`Request failed: ${e}`);
            return null;
        }
    }
  
    protected async requestBytes(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<Blob | null> {
        try {
            const response = await this._fetch(endpoint, options);
        
            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || 'API request failed');
            }
            return response.blob();
        } catch (e) {
            console.log(`Request failed: ${e}`);
            return null;
        }
    }
  
    protected async requestString(
        endpoint: string,
        options: RequestInit = {}
    ): Promise<string | null> {
        try {
            const response = await this._fetch(endpoint, options);
    
            if (!response.ok) {
                const error = await response.json();
                throw new Error(error.message || 'API request failed');
            }
        
            return response.text();
        } catch (e) {
            console.log(`Request failed: ${e}`);
            return null;
        }
    }
}
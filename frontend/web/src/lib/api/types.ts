export interface SFile {
    is_dir: boolean,
    full_path: String,
    created_at: BigInt,
    modified_at: BigInt,
    // Either the name of the directory or the file
    top_level_name: String
}

export class BaseClient {
    protected serverUrl: string;
    protected headers: Record<string, string>;
  
    constructor() {
        const url = localStorage.getItem("OCLOUD_URL");
        if (!url) 
            throw new Error("No server url, could not create api wrapper.");

        this.serverUrl = url;
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
// WebSocket related types - using short field names (t/d) as per API spec
export interface FileCreatedEvent {
    t: "FileCreated";
    d: {
        path: string;
        file_id: number;
        is_dir: boolean;
    };
}

export interface FileDeletedEvent {
    t: "FileDeleted";
    d: {
        path: string;
        file_id: number;
    };
}

export interface FileMovedEvent {
    t: "FileMoved";
    d: {
        from_path: string;
        to_path: string;
        file_id: number;
    };
}

export interface UploadProgressEvent {
    t: "UploadProgress";
    d: {
        path: string;
        file_name: string;
        bytes_uploaded: number;
        total_bytes: number;
        progress_percent: number;
    };
}

export interface UploadCompletedEvent {
    t: "UploadCompleted";
    d: {
        path: string;
        file_name: string;
        file_id: number;
    };
}

export type WebSocketEvent = 
    | FileCreatedEvent
    | FileDeletedEvent
    | FileMovedEvent
    | UploadProgressEvent
    | UploadCompletedEvent;

export interface CancelUploadRequest {
    t: "CancelUpload";
    d: {
        temp: string;
    };
}

export class WebSocketApi {
    private ws: WebSocket | null = null;
    private serverUrl: string;
    private listeners: Map<string, ((event: WebSocketEvent) => void)[]> = new Map();
    private reconnectAttempts = 0;
    private maxReconnectAttempts = 5;
    private reconnectDelay = 1000; // Start with 1 second
    private isConnected = false;

    constructor(serverUrl: string) {
        // Convert HTTP URL to WebSocket URL
        this.serverUrl = serverUrl.replace(/^https?:\/\//, 'ws://').replace(/^http:\/\//, 'ws://');
        if (this.serverUrl.startsWith('https://')) {
            this.serverUrl = this.serverUrl.replace('https://', 'wss://');
        }
    }

    connect(): Promise<void> {
        return new Promise((resolve, reject) => {
            try {
                this.ws = new WebSocket(`${this.serverUrl}/ws`);

                this.ws.onopen = () => {
                    console.log('WebSocket connected');
                    this.isConnected = true;
                    this.reconnectAttempts = 0;
                    this.reconnectDelay = 1000;
                    resolve();
                };

                this.ws.onmessage = (event) => {
                    try {
                        const data: WebSocketEvent = JSON.parse(event.data);
                        this.handleEvent(data);
                    } catch (error) {
                        console.error('Failed to parse WebSocket message:', error);
                    }
                };

                this.ws.onclose = () => {
                    console.log('WebSocket disconnected');
                    this.isConnected = false;
                    this.attemptReconnect();
                };

                this.ws.onerror = (error) => {
                    console.error('WebSocket error:', error);
                    if (!this.isConnected) {
                        reject(error);
                    }
                };
            } catch (error) {
                reject(error);
            }
        });
    }

    private attemptReconnect() {
        if (this.reconnectAttempts >= this.maxReconnectAttempts) {
            console.error('Max reconnection attempts reached');
            return;
        }

        this.reconnectAttempts++;
        console.log(`Attempting to reconnect... (${this.reconnectAttempts}/${this.maxReconnectAttempts})`);

        setTimeout(() => {
            this.connect().catch(error => {
                console.error('Reconnection failed:', error);
                this.reconnectDelay *= 2; // Exponential backoff
            });
        }, this.reconnectDelay);
    }

    private handleEvent(event: WebSocketEvent) {
        const listeners = this.listeners.get(event.t) || [];
        listeners.forEach(listener => listener(event));

        // Also notify wildcard listeners
        const wildcardListeners = this.listeners.get('*') || [];
        wildcardListeners.forEach(listener => listener(event));
    }

    addEventListener(eventType: string, listener: (event: WebSocketEvent) => void) {
        if (!this.listeners.has(eventType)) {
            this.listeners.set(eventType, []);
        }
        this.listeners.get(eventType)!.push(listener);
    }

    removeEventListener(eventType: string, listener: (event: WebSocketEvent) => void) {
        const listeners = this.listeners.get(eventType);
        if (listeners) {
            const index = listeners.indexOf(listener);
            if (index > -1) {
                listeners.splice(index, 1);
            }
        }
    }

    sendCancelUpload(temp: string) {
        if (this.ws && this.isConnected) {
            const request: CancelUploadRequest = {
                t: "CancelUpload",
                d: { temp }
            };
            this.ws.send(JSON.stringify(request));
        }
    }

    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
        this.isConnected = false;
    }

    getConnectionState(): boolean {
        return this.isConnected;
    }
}
import { AuthApi } from "./auth";
import { MediaApi } from "./media";
import { TestApi } from "./test";
import { WebSocketApi } from "./websocket";

/**
 * Comprehensive API client for the OCloud server
 */
export class OCloudApi {
    private serverUrl: string;
    
    // Individual API clients
    public readonly auth: AuthApi;
    public readonly media: MediaApi;
    public readonly test: TestApi;
    public readonly websocket: WebSocketApi;

    constructor(serverUrl: string = "http://localhost:8000") {
        this.serverUrl = serverUrl;
        
        // Initialize all API clients
        this.auth = new AuthApi(serverUrl);
        this.media = new MediaApi(serverUrl);
        this.test = new TestApi(serverUrl);
        this.websocket = new WebSocketApi(serverUrl);
    }

    /**
     * Initialize the media cache
     */
    async initMediaCache(): Promise<boolean> {
        return this.media.initCache();
    }

    /**
     * Connect to WebSocket for real-time events
     */
    async connectWebSocket(): Promise<void> {
        return this.websocket.connect();
    }

    /**
     * Disconnect from WebSocket
     */
    disconnectWebSocket(): void {
        this.websocket.disconnect();
    }

    /**
     * Get the server URL
     */
    getServerUrl(): string {
        return this.serverUrl;
    }

    /**
     * Set authentication token
     */
    setAuthToken(token: string): void {
        localStorage.setItem('OCLOUD_AUTH', token);
    }

    /**
     * Get current authentication token
     */
    getAuthToken(): string | null {
        return localStorage.getItem('OCLOUD_AUTH');
    }

    /**
     * Clear authentication token
     */
    clearAuthToken(): void {
        localStorage.removeItem('OCLOUD_AUTH');
    }

    /**
     * Check if user is authenticated
     */
    isAuthenticated(): boolean {
        return this.getAuthToken() !== null;
    }

    /**
     * Complete authentication flow - stores token and can initialize WebSocket
     */
    async authenticate(token: string, initWebSocket: boolean = true): Promise<void> {
        this.setAuthToken(token);
        
        if (initWebSocket) {
            try {
                await this.connectWebSocket();
            } catch (error) {
                console.warn('Failed to connect WebSocket after authentication:', error);
            }
        }
    }

    /**
     * Complete logout flow - clears token and disconnects WebSocket
     */
    async logout(): Promise<boolean> {
        try {
            const success = await this.auth.logout();
            this.clearAuthToken();
            this.disconnectWebSocket();
            return success;
        } catch (error) {
            // Even if logout request fails, clear local state
            this.clearAuthToken();
            this.disconnectWebSocket();
            return false;
        }
    }

    /**
     * Health check endpoints
     */
    async health(): Promise<boolean> {
        try {
            const response = await fetch(`${this.serverUrl}/health`);
            return response.ok;
        } catch {
            return false;
        }
    }

    async ping(): Promise<string | null> {
        return this.test.ping();
    }
}

// Export a default instance for convenience
export const apiClient = new OCloudApi();

// Export all types for external use
export * from "./auth";
export * from "./media";
export * from "./websocket";
export * from "./path";
export * from "./types";
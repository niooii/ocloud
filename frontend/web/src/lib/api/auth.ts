import { BaseClient } from "./types";

// Authentication related types - matches Rust UserInfo
export interface User {
    id: number;
    username: string;
    email: string;
    created_at: string;
    last_login?: string;
}

export interface AuthResponse {
    user: User;
    session_id: string;
}

export interface RegisterRequest {
    username: string;
    email: string;
    password: string;
}

export interface LoginRequest {
    username: string;
    password: string;
}

// Permission related types
export type RelationshipType = "owner" | "editor" | "viewer";

export interface PermissionGrantRequest {
    target_user_id: number;
    resource_type: string;
    resource_id?: number;
    relationship: RelationshipType;
    expires_at?: string;
}

export interface PermissionRevokeRequest {
    target_user_id: number;
    resource_type: string;
    resource_id?: number;
    relationship: RelationshipType;
}

export interface PermissionInfo {
    user: User;
    relationship: RelationshipType;
    granted_by?: number;
    granted_at: string;
    expires_at?: string;
}

export class AuthApi extends BaseClient {
    async register(request: RegisterRequest): Promise<AuthResponse | null> {
        return this.request<AuthResponse>(
            '/auth/register',
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(request),
            }
        );
    }

    async login(request: LoginRequest): Promise<AuthResponse | null> {
        return this.request<AuthResponse>(
            '/auth/login',
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(request),
            }
        );
    }

    async logout(): Promise<boolean> {
        const response = await this.request(
            '/auth/logout',
            {
                method: 'POST',
            }
        );
        return response !== null;
    }

    async getMe(): Promise<User | null> {
        return this.request<User>('/auth/me', {
            method: 'GET',
        });
    }

    async grantPermission(request: PermissionGrantRequest): Promise<boolean> {
        const response = await this.request(
            '/auth/permissions/grant',
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(request),
            }
        );
        return response !== null;
    }

    async revokePermission(request: PermissionRevokeRequest): Promise<boolean> {
        const response = await this.request(
            '/auth/permissions/revoke',
            {
                method: 'POST',
                headers: {
                    'Content-Type': 'application/json',
                },
                body: JSON.stringify(request),
            }
        );
        return response !== null;
    }

    async getPermissions(resourceType: string): Promise<PermissionInfo[] | null> {
        return this.request<PermissionInfo[]>(
            `/auth/permissions/${resourceType}`,
            {
                method: 'GET',
            }
        );
    }

    async getResourcePermissions(
        resourceType: string, 
        resourceId: number
    ): Promise<PermissionInfo[] | null> {
        return this.request<PermissionInfo[]>(
            `/auth/permissions/${resourceType}/${resourceId}`,
            {
                method: 'GET',
            }
        );
    }
}
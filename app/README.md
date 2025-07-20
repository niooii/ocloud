# ocloud

A cloud file storage server, with some extras

## Quick Start

```bash
docker-compose up -d        # Start database
sqlx migrate run           # Run migrations  
cargo run -- server run   # Start server (localhost:8000)
```

## API

### Health Endpoints
- `GET /health` - Health check (empty response)
- `GET /ping` - Returns "pong...?"

Example: `curl http://localhost:8000/health`

### File Management

**Note: All paths must start with "root/", not "/root/" or "/"**

#### `GET /files/[path]`
**File** - Returns the binary contents of the file. Content type depends on the file's extension.  
**Directory** - Lists the directory contents. Returns a JSON array of files.

Note: path is a **directory** if it ends with '/'.

Example: `curl http://localhost:8000/files/root/my-folder/`

#### `POST /files/root/[dir]` 
**Directory**: 
- Posts the *first* file sent in the form only. Send multiple requests to post multiple files. (TODO! fix this this is horrible) Returns the new file in a JSON array of length 1.
- If there is no request body or file in the form, **creates all the immediate directories** and returns the directories newly created in a JSON array.

All immediate directories are created upon any action.

Example: `curl -X POST http://localhost:8000/files/root/folder/ -F "file=@myfile.txt"`

#### `DELETE /files/[path]`
**File** - Deletes the file. Returns nothing.

Example: `curl -X DELETE http://localhost:8000/files/root/myfile.txt`

#### `PUT /files`
Move/rename files. Request body: `{"from": "root/old/path", "to": "root/new/path"}`

Example: `curl -X PUT http://localhost:8000/files -d '{"from":"root/a.txt","to":"root/b.txt"}' -H "Content-Type: application/json"`

#### `PATCH /files` (Protected)
Change file visibility. Request body: `{"path": "root/file.txt", "visibility": "public"}` or `{"path": "root/file.txt", "visibility": "private"}`

Example: `curl -X PATCH http://localhost:8000/files -d '{"path":"root/file.txt","visibility":"public"}' -H "Content-Type: application/json" -H "Authorization: Bearer <session_id>"`

### Authentication

#### `POST /auth/register`
Register a new user. Request body:
```json
{
  "username": "user",
  "email": "user@example.com", 
  "password": "password"
}
```

Returns user info and session ID.

Example:
```bash
curl -X POST http://localhost:8000/auth/register \
  -d '{
    "username": "testuser",
    "email": "test@example.com",
    "password": "password123"
  }' \
  -H "Content-Type: application/json"
```

#### `POST /auth/login`
Login with username/email and password. Request body:
```json
{
  "username": "user",
  "password": "password"
}
```

Returns user info and session ID for Bearer token authentication.

Example:
```bash
curl -X POST http://localhost:8000/auth/login \
  -d '{
    "username": "testuser",
    "password": "password123"
  }' \
  -H "Content-Type: application/json"
```

#### `GET /auth/me` (Protected)
Get current user info and permissions. Requires `Authorization: Bearer <session_id>` header.

Example:
```bash
curl http://localhost:8000/auth/me \
  -H "Authorization: Bearer <session_id>"
```

#### `POST /auth/logout` (Protected)
Logout and invalidate session. Requires `Authorization: Bearer <session_id>` header.

Example:
```bash
curl -X POST http://localhost:8000/auth/logout \
  -H "Authorization: Bearer <session_id>"
```

#### `POST /auth/permissions/grant` (Protected)
Grant permissions to a user for a resource. Request body:
```json
{
  "target_user_id": 123,
  "resource_type": "sfile",
  "resource_id": 456,
  "relationship": "editor"
}
```

Relationship types: `owner`, `editor`, `viewer`

#### `POST /auth/permissions/revoke` (Protected)
Revoke permissions from a user. Request body:
```json
{
  "target_user_id": 123,
  "resource_type": "sfile",
  "resource_id": 456,
  "relationship": "editor"
}
```

#### `GET /auth/permissions/{resource_type}` (Protected)
View permissions for all resources of a type.

#### `GET /auth/permissions/{resource_type}/{resource_id}` (Protected)
View permissions for a specific resource.

### WebSocket Real-time Events

#### `WS /ws`
Connect to receive real-time file system events and upload progress.

Events (JSON):
- `FileCreated` - File/directory created: `{"type": "FileCreated", "data": {"path": "/file.txt", "file_id": 123, "is_dir": false}}`
- `FileDeleted` - File deleted: `{"type": "FileDeleted", "data": {"path": "/file.txt", "file_id": 123}}`  
- `FileMoved` - File moved/renamed: `{"type": "FileMoved", "data": {"from_path": "/old.txt", "to_path": "/new.txt", "file_id": 123}}`
- `UploadProgress` - Upload progress: `{"type": "UploadProgress", "data": {"path": "/folder/", "file_name": "big.zip", "bytes_uploaded": 1024, "total_bytes": 2048, "progress_percent": 50.0}}`

Example: 
```javascript
const ws = new WebSocket('ws://localhost:8000/ws');
ws.onmessage = (event) => console.log(JSON.parse(event.data));
```

## TODO
~~- AUTH????????? (kinda and ReBAC)
- tie each websocket connection to a user id (or not, depending on my next point)
- default create a public folder where all files are available to everyone, even unauthenticated users (useful for sharing stuff). ~~
- TODO tests kinda pollute postgres with random databases if they panic.. panic hook?
- Well the test config is kinda useless ngl, make it useful or remove it
- for each [file] uploaded, store a [file].meta that contains all the names the media is aliased under. can be used to restore the database and easily download everything
- Add multi upload
- Make frontend handle the displaying of videos and images like medal
- Optional MD document editor in the frontend, google docs kinda
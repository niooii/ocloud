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

#### `GET /files/[path]`
**File** - Returns the binary contents of the file. Content type depends on the file's extension.  
**Directory** - Lists the directory contents. Returns a JSON array of files.

Note: path is a **directory** if it ends with '/'.

Example: `curl http://localhost:8000/files/root/my-folder/`

#### `POST /files/[path]` 
**Directory**: 
- Posts the *first* file sent in the form only. Send multiple requests to post multiple files. (TODO! fix this this is horrible) Returns the new file in a JSON array of length 1.
- If there is no request body or file in the form, **creates all the immediate directories** and returns the directories newly created in a JSON array.

All immediate directories are created upon any action.

Example: `curl -X POST http://localhost:8000/files/root/folder/ -F "file=@myfile.txt"`

#### `DELETE /files/[path]`
**File** - Deletes the file. Returns nothing.

Example: `curl -X DELETE http://localhost:8000/files/root/myfile.txt`

#### `PATCH /files/*`
Move/rename files. Request body: `{"from": "/old/path", "to": "/new/path"}`

Example: `curl -X PATCH http://localhost:8000/files/root -d '{"from":"/a.txt","to":"/b.txt"}' -H "Content-Type: application/json"`

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
- AUTH????????? (kinda and ReBAC)
- tie each websocket connection to a user id (or not, depending on my next point)
- default create a public folder where all files are available to everyone, even unauthenticated users (useful for sharing stuff). 
- for each [file] uploaded, store a [file].meta that contains all the names the media is aliased under. can be used to restore the database and easily download everything
- Add multi upload
- Make frontend handle the displaying of videos and images like medal
- Optional MD document editor in the frontend, google docs kinda
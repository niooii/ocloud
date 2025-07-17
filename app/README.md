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

Example: `curl http://localhost:8000/files/my-folder/`

#### `POST /files/[path]` 
**Directory**: 
- Posts the *first* file sent in the form only. Send multiple requests to post multiple files. (TODO! fix this this is horrible) Returns the new file in a JSON array of length 1.
- If there is no request body or file in the form, **creates all the immediate directories** and returns the directories newly created in a JSON array.

All immediate directories are created upon any action.

Example: `curl -X POST http://localhost:8000/files/folder/ -F "file=@myfile.txt"`

#### `DELETE /files/[path]`
**File** - Deletes the file. Returns nothing.

Example: `curl -X DELETE http://localhost:8000/files/myfile.txt`

#### `PATCH /files`
Move/rename files. Request body: `{"from": "/old/path", "to": "/new/path"}`

Example: `curl -X PATCH http://localhost:8000/files -d '{"from":"/a.txt","to":"/b.txt"}' -H "Content-Type: application/json"`

See [DOCUMENTATION.md](./DOCUMENTATION.md) for detailed development info.

## TODO
- AUTH?????????
- for each [file] uploaded, store a [file].meta that contains all the names the media is aliased under. can be used to restore the database and easily download everything
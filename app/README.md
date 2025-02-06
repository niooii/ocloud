# NOTE
DO NOT install this.  
This is actively being worked on, and I published it to make sure it compiles correctly  

# Environment
Rename `example.env` -> `.env`, then fill out the variables accordingly.

# API
## /files/[path]
Note - path is a **directory** if it ends with '/'.\
All immediate directories are created upon any action.  

### `GET`
**File** - Returns the binary contents of the file.  Content type depends on the file's extension.  
**Directory** - Lists the directory contents.  Returns a JSON array of files.
### `DELETE`
**File** - Deletes the file.  Returns nothing.
### `POST`
**Directory**: 
- Posts the *first* file sent in the form only. Send multiple requests to post multiple files. (TODO! this is stupid change it). Returns the new file in a JSON array of length 1.
- If there is no request body or file in the form, **creates all the immediate directories** and returns the directories newly created in a JSON array.

# TODO
- AUTH?????????
- for each [file] uploaded, store a [file].meta that contains all the names the media is aliased under. can be used to restore the database and easily download everything
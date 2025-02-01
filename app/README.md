# NOTE
DO NOT install this.  
This is actively being worked on, and I published it to make sure it compiles correctly  

# Environment
Rename `example.env` -> `.env`, then fill out the variables accordingly.

# API
## /media/[path]
Note - path is a **directory** if it ends with '/'.\
All directories are created upon access.  

`GET` (file/dir) - Retrieves the file, or lists the directory contents\
`DELETE` (file) - Deletes the file\
`POST` (dir) - Uploads a file to the directory. (sent via multipart)\

# TODO
- for each [file] uploaded, store a [file].meta that contains all the names the media is aliased under. can be used to restore the database and download everything
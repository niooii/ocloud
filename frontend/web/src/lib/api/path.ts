const ROOT = "root"
const ROOT_DIR = "root/"

export class Path {
    private pathStr: string;
    
    // Uses front slashes
    constructor(pathString: string) {
        if (pathString.length === 0)
            throw new Error("Path cannot be an empty string.");
        if (pathString.startsWith("/"))
            throw new Error("Path cannot start with '/'. If you're trying to create a root path, use 'root/' instead, or the static .root() method.");
        this.pathStr = pathString;
        this.clean();
    }

    public static root(): Path {
        return new Path(ROOT_DIR);
    }

    public toString(): string {
        // force deep copy
        return `${this.pathStr}`;
    }

    // Returns a new path object from the current one, not a ref.
    public clone() {
        return new Path(this.toString());
    }

    // Sets the current path to the one passed in
    public set(to: Path) {
        this.pathStr = to.toString();
    }
    
    // A path is considered absolute if it starts with the root: "root/"
    public isAbsolute(): boolean {
        return this.pathStr.startsWith(ROOT_DIR);
    }

    public isRoot(): boolean {
        return this.pathStr === ROOT_DIR;
    }

    // A path is considered relative if it does not start with the root: "root/"
    public isRelative(): boolean {
        return !this.isAbsolute();
    }

    // A path is a `directory` if it ends with a forward slash
    public isDir(): boolean {
        return this.pathStr.endsWith("/");
    }
    
    // A path is a `directory` if it does not end with a forward slash
    public isFile(): boolean {
        return !this.isDir();
    }

    // Converts the current path into a file
    public intoFile() {
        if (this.isDir()) {
            this.pathStr = this.pathStr.slice(0, -1);
        }
    }

    // Converts the current path into a directory
    public intoDir() {
        if (this.isFile()) {
            this.pathStr += "/"
        }
    }

    // Returns a new copy of the Path, as a file
    public asFile(): Path {
        let clone = this.clone();
        clone.intoFile();
        return clone;
    }
    
    // Returns a new copy of the Path, as a directory
    public asDir(): Path {
        let clone = this.clone();
        clone.intoDir();
        return clone;
    }

    // Includes the root path if applicable.
    public getPathParts(): string[] {
        let parts = this.pathStr.split("/");
        return parts;
    }

    // Removes trailing slashes and duplicate consecutive slashes, as well as any illegal characters.
    private clean() {
        if (this.pathStr.length > 1) {
            // convert back to forward
            this.pathStr = this.pathStr.replace(/\\/g, '/');
            
            // remove consecutive duplicate slashes
            this.pathStr = this.pathStr.replace(/\/+/g, '/');
            
            // trailing slashes
            const endsWithSlash = this.pathStr.endsWith('/');
            this.pathStr = this.pathStr.replace(/\/+$/, '');
            if (endsWithSlash) {
                this.pathStr += '/';
            }
        }
    }

    public equals(other: Path): boolean {
        return other.pathStr === this.pathStr;
    }

    // Returns undefined if the path is the root directory, or if the parent
    // of the relative path cannot be known.
    public parent(): Path | undefined {
        if (this.isRoot()) {
            return undefined;
        }

        let path = this.pathStr;
        
        let targetSlashIdx;
        if (this.isDir()) {
            // last slash is a front slash, so find the second one.
            targetSlashIdx = path.lastIndexOf("/", path.length - 2);
        } else {
            targetSlashIdx = path.lastIndexOf("/");
        }
        
        // if no slashes (rel file/dir)
        if (targetSlashIdx === -1) {
            return undefined;
        }

        return new Path(path.substring(0, targetSlashIdx));
    }

    // Returns the new joined path. Returns undefined if the path `other` is absolute, 
    // or if the current path is not a directory.
    public join(other: Path): Path | undefined {
        let path = this.pathStr;

        if (other.isAbsolute() || !this.isDir()) {
            return undefined;
        }

        let newPath = new Path(`${this.toString()}/${other.toString}`);
        return newPath;
    }

    // Returns the new joined path. Returns undefined if the path `other` is absolute, 
    // or if the current path is not a directory.
    public join_str(other: string): Path | undefined {
        let path = this.pathStr;
        let other_path = new Path(other);

        if (other_path.isAbsolute() || !this.isDir()) {
            return undefined;
        }

        let newPath = new Path(`${this.toString()}/${other_path.toString()}`);
        return newPath;
    }

    public name(): string {
        let path = this.pathStr;
        if (this.isRoot()) {
            return ROOT;
        }
        const lastSlashIndex = path.lastIndexOf("/");
        if (lastSlashIndex === -1) {
            return path;
        }
        if (this.isDir()) {
            const secondLastSlash = path.lastIndexOf("/", path.length-2);
            return path.substring(secondLastSlash + 1, path.length-1);
        } else {
            return path.substring(lastSlashIndex + 1);
        }
    }

    public push(other: Path) {
        if (other.isAbsolute()) {
            throw new Error("Cannot push an absolute path");
        }

        let basePath = this.pathStr;
        
        this.pathStr = `${basePath}/${other.pathStr}`;
        this.clean();
    }

    public pop() {
        const parent = this.parent();
        if (parent === undefined) {
            throw new Error("Cannot pop from root or invalid path");
        }
        
        this.pathStr = parent.pathStr;
    }
}
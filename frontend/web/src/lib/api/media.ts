import { Path } from "./path";
import { BaseClient } from "./types";
import { SFile } from "./types";

interface SFileRaw {
    id: number,
    is_dir: boolean,
    full_path: string,
    created_at: string,
    modified_at: string,
    // Either the name of the directory or the file
    top_level_name: string
}

export class MediaApi extends BaseClient {
    async listDir(dir: Path): Promise<SFile[]> {
        const _dir = dir.asDir();
        const raw = await this.request<SFileRaw[]>(
            `/files/${_dir.toString()}`,
            {
                method: "GET"
            }
        );
        return raw.map((raw): SFile => {
            return {
                id: raw.id,
                isDir: raw.is_dir,
                fullPath: new Path(raw.full_path),
                createdAt: new Date(raw.created_at),
                modifiedAt: new Date(raw.modified_at),
                // Either the name of the directory or the file
                topLevelName: raw.top_level_name
            };
        });
    }

  async uploadFile(dir: Path, file: File): Promise<null> {
    const uploadPath = dir.joinStr(file.name);
    const result = await this.requestString(
        `/media/${uploadPath}`, 
        {
            method: "POST",
            // how to send multipart
        }
    );

    return null;
  }
}
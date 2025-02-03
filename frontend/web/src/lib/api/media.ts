import { Path } from "./path";
import { BaseClient } from "./types";
import { SFile } from "./types";

export class MediaApi extends BaseClient {
    async listDir(dir: Path): Promise<[SFile]> {
        const _dir = dir.asDir();
        return this.request<[SFile]>(
            `/media/${_dir.toString()}`,
            {
                method: "GET"
            }
        );
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
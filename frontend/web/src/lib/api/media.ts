import { Path } from "./path";
import { BaseClient } from "./types";
import { SFile } from "./types";

export class MediaApi extends BaseClient {
    async listDir(dir: Path): Promise<[SFile]> {
        return this.request<[SFile]>(
            `/media/${dir.toString()}`,
            {
                method: "GET"
            }
        );
    }

  async uploadFile(dir: Path, file: File): Promise<null> {
    const uploadPath = dir.join_str(file.name);
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
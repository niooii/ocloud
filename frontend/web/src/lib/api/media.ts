import { Path } from "./path";
import { BaseClient } from "./types";
import { SFile } from "./types";

export class MediaApi extends BaseClient {
  async listDir(dir: Path): Promise<[SFile]> {
    return this.request<[SFile]>(dir.toString());
  }

  async uploadFile(dir: Path, file: File): Promise<null> {
    const uploadPath = dir.join_str(file.name);
    const uploadData = await this.request<string>(`${this.baseUrl}/media/${uploadPath}`, {
      method: 'POST',
      body: JSON.stringify({
        filename: file.name,
        size: file.size,
      }),
    });

    return null;
  }
}
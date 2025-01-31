import { BaseClient } from "./types";

export class TestApi extends BaseClient {
    async ping(): Promise<string> {
        const pong = await this.requestString(
            `/ping`, 
            {
                method: "GET",
            }
        );
    
        return pong;
    }
}
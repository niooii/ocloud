import { redirect } from "next/navigation";
import { TestApi } from "./api/test";

export async function ping(serverUrl: string): Promise<boolean> {
    try {
        let test = new TestApi(serverUrl);
        const res = await test.ping();

        if (res === "pong...?") {
            return true;
        } else {
            return false;
        }
    } 
    catch (e) {
        console.error("Error during ping: ", e);
        return false;
    }
}

export function getServerUrl(): string | null {
    return localStorage.getItem("OCLOUD_URL");
}

export function saveServerUrl(serverUrl: string) {
    localStorage.setItem("OCLOUD_URL", serverUrl);
}

export function clearServerUrl() {
    localStorage.removeItem("OCLOUD_URL");
}
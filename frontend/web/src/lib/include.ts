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

// Will redirect to home page if the serverUrl is null.
export function getServerUrl(): string {
    const url = localStorage.getItem("OCLOUD_URL");
    if (!url) {
        console.log("Could not find server url for some reason, redirecting to landing page.");
        redirect("/");
    }
    return url;
}

export function saveServerUrl(serverUrl: string) {
    localStorage.setItem("OCLOUD_URL", serverUrl);
}

export function clearServerUrl() {
    localStorage.removeItem("OCLOUD_URL");
}
"use client"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { TestApi } from "@/lib/api/test"
import { useEffect, useState } from "react"

export default function LandingForm() {
    const [urlValue, setUrlValue] = useState("")

    const handleURLSubmission = async () => {
        if (urlValue) {
            console.log("Submitting URL:", urlValue)
            const cleanUrl = urlValue.replace(/\/$/, '');
            localStorage.setItem("OCLOUD_URL", cleanUrl);
        }

        try {
            let test = new TestApi();
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

    useEffect(() => {
        const savedUrl = localStorage.getItem("OCLOUD_URL")
        if (savedUrl) setUrlValue(savedUrl)
    }, [])

    return (
        <>
        <Input 
            type="url" 
            placeholder="Server URL (https://yourdomain.com)"
            className="text-center"
            onChange={(e) => setUrlValue(e.target.value)}
            defaultValue={urlValue}
        />
        <Button 
            className={`transition-all duration-300 ${
                urlValue ? "opacity-100 translate-y-1" : "opacity-0 translate-y-0 invisible pointer-events-none"
            }`}
            variant="outline"
            type="submit"
            onClick={(_e) => handleURLSubmission().then(pinged => {
                if (pinged) {
                    console.log("ping success.. url is valid");
                } else {
                    console.log("ping failed.. bad url");
                }
            })
            .catch(e => console.log("err: " + e))}
        > 
            Enter..
        </Button>
        </>
    )
}
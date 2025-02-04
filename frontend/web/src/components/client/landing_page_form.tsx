"use client"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { TestApi } from "@/lib/api/test"
import { redirect } from "next/navigation"
import { useEffect, useState } from "react"
import { ToastAction } from "@/components/ui/toast"
import { useToast } from "@/hooks/use-toast"
import { clearServerUrl, errorToast, getServerUrl, ping, saveServerUrl } from "@/lib/include";

export default function LandingForm() {
    const [urlValue, setUrlValue] = useState("");
    const { toast } = useToast();

    useEffect(() => {
        const cachedUrl = getServerUrl();

        if (!cachedUrl) {
            return;
        }

        ping(cachedUrl).then((pinged) => {
            if (pinged) {
                redirect("/home");
            } else {
                errorToast(
                    "Somethings off...",
                    "Check your URL or server and try again.",
                );
                clearServerUrl();
            }
        })
    }, [])

    const handleURLSubmission = () => {
        if (urlValue) {
            const cleanUrl = urlValue.replace(/\/$/, '');
            ping(cleanUrl).then((pinged) => {
                if (pinged) {
                    saveServerUrl(cleanUrl);
                    // redirect blah blah\
                    toast({
                        variant: "default",
                        title: "Zoooooom",
                        description: "Reached server successfully!",
                        duration: 2000
                    });
                    redirect("/home");
                } else {
                    // failed to reach the server url
                    errorToast(
                        "Could not reach the URL provided :(",
                        "Check your URL or server and try again.",
                    );
                }
            })
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
            className="text-center max-w-md mx-auto"
            onChange={(e) => setUrlValue(e.target.value)}
            defaultValue={urlValue}
        />
        <Button 
            className={`transition-all duration-300 ${
                urlValue ? "opacity-100 translate-y-1" : "opacity-0 translate-y-0 invisible pointer-events-none"
            }`}
            variant="outline"
            type="submit"
            onClick={(_e) => handleURLSubmission()}
        > 
            Enter..
        </Button>
        </>
    )
}
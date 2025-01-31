"use client"

import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"
import { useEffect, useState } from "react"

export default function LandingForm() {
    const [urlValue, setUrlValue] = useState("")

    const handleURLSubmission = () => {
        if (urlValue) {
            console.log("Submitting URL:", urlValue)
            const cleanUrl = urlValue.replace(/\/$/, '');
            localStorage.setItem("OCLOUD_URL", cleanUrl);
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
            className="text-center text-xl3"
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
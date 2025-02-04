"use client"

import { Input } from "@/components/ui/input";
import { MediaApi } from "@/lib/api/media";
import { Path } from "@/lib/api/path";
import { getServerUrl } from "@/lib/include";
import { ChangeEvent } from "react";

export default function FileUploader() {
    
    const handleUploadFile = (e: ChangeEvent<HTMLInputElement>) => {
        const api = new MediaApi(getServerUrl()!);
        api.uploadFile(Path.root(), e.target.files![0])
    };

    return (
        <div>
            <Input type="file" placeholder="asfas" name="AWfa" onChange={
                    (e) => {
                        handleUploadFile(e) 
                    }
                }
            />
        </div>
    );
}
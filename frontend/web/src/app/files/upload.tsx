"use client"

import { Input } from "@/components/ui/input";
import { ChangeEvent } from "react";

export default function FileUploader() {
    
    const handleUploadFile = (e: ChangeEvent<HTMLInputElement>) => {
        console.log("UPLOADED !!");

        // const mediaApi = new MediaApi();
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
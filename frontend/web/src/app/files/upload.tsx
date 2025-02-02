"use client"

import { Input } from "@/components/ui/input";

export default function FileUploader() {
    // TODO! figure out the type
    const handleUploadFile = (e) => {
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
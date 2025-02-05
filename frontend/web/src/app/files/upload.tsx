"use client"

import { Button } from "@/components/ui/button";
import { MediaApi } from "@/lib/api/media";
import { Path } from "@/lib/api/path";
import { getServerUrl } from "@/lib/include";
import { ChangeEvent, useRef, useState } from "react";

interface FileUploaderProps {
    onChanged?: (files: FileList) => void;
}

export default function FileUploader({ onChanged }: FileUploaderProps) {
    const fileInputRef = useRef<HTMLInputElement>(null);

    const handleUploadFile = (e: ChangeEvent<HTMLInputElement>) => {
        if (onChanged && e.target.files) {
            onChanged(e.target.files);
        }
    };

    const handleButtonClick = () => {
        fileInputRef.current?.click();
    };

    return (
        <div className="relative">
            <input
                ref={fileInputRef}
                type="file"
                onChange={handleUploadFile}
                className="hidden"
                name="a"
            />
            
            <Button 
                onClick={handleButtonClick}
                variant="outline"
                className="w-full"
            >
            Upload some files!
            </Button>
        </div>
    );
}
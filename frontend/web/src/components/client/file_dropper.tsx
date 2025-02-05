"use client"

import React from 'react';
import { useDropzone } from 'react-dropzone';

interface FileDropAreaProps {
    children: React.ReactNode;
    onFileUpload: (files: File[]) => void;
}

const FileDropArea: React.FC<FileDropAreaProps> = ({ children, onFileUpload }) => {
    const { getRootProps, getInputProps, isDragActive } = useDropzone({
        onDrop: (acceptedFiles) => {
            onFileUpload(acceptedFiles);
        },
        useFsAccessApi: false,
        preventDropOnDocument: true,
        noClick: true,
        noKeyboard: true,
    });

    return (
        <div
            {...getRootProps()}
            className={`contents ${
                isDragActive ? 'bg-blue-50' : ''
            }`}
        >
            <input {...getInputProps()} />
            
            {children}
        </div>
    );
};

export default FileDropArea;
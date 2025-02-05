"use client"

import React, { useState, useCallback } from 'react';

interface FileDropAreaProps {
    children: React.ReactNode;
    onFileUpload: (files: File[]) => void;
}

const FileDropArea: React.FC<FileDropAreaProps> = ({ children, onFileUpload }) => {
    const [isDragging, setIsDragging] = useState(false);

    const onDragOver = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDragging(true);
    }, []);

    const onDragLeave = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDragging(false);
    }, []);

    const onDrop = useCallback((e: React.DragEvent) => {
        e.preventDefault();
        e.stopPropagation();
        setIsDragging(false);

        const files = Array.from(e.dataTransfer.files);
        if (files.length > 0) {
        onFileUpload(files);
        }
    }, [onFileUpload]);

    return (
        <div
        onDragOver={onDragOver}
        onDragLeave={onDragLeave}
        onDrop={onDrop}
        className={`contents ${isDragging ? 'bg-blue-50' : ''}`}
        >
        {isDragging && (
            <div className="absolute inset-0 border-2 border-dashed border-blue-500 bg-blue-50 bg-opacity-50 flex items-center justify-center">
            <p className="text-blue-500">Drop files here</p>
            </div>
        )}
        {children}
        </div>
    );
};

export default FileDropArea;
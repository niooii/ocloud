import React from "react";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { FileText, Image as ImageIcon, File, Loader2 } from "lucide-react";
import { getFileIcon } from "./utils";
import ReactPlayer from 'react-player/lazy'

interface BlobViewerProps {
  future: Promise<Blob | null>;
  filename?: string;
}

export default function MediaViewer({ future, filename }: BlobViewerProps) {
  const [state, setState] = React.useState<"loading" | "error" | "success">("loading");
  const [content, setContent] = React.useState<string | null>(null);
  const [type, setType] = React.useState<"image" | "text" | "video" | "other">("other");
  const [blob, setBlob] = React.useState<Blob | null>(null);

  React.useEffect(() => {
    const loadBlob = async () => {
      try {
        const result = await future;
        if (!result) {
          setState("error");
          return;
        }
        setBlob(result);
        
        if (result.type.startsWith("image/")) {
          setType("image");
        } else if (result.type.startsWith("video/")) {
          setType("video");
        } 
        else if (result.type.startsWith("text/") || result.type === "application/json") {
          setType("text");
          const text = await result.text();
          setContent(text);
        }
        setState("success");
      } catch (e) {
        setState("error");
      }
    };

    loadBlob();
  }, [future]);

  const renderContent = () => {
    switch (state) {
      case "loading":
        return (
          <div className="flex justify-center items-center p-12">
            <Loader2 className="h-8 w-8 animate-spin" />
          </div>
        );
      case "error":
        return (
          <div className="flex justify-center items-center p-12 text-red-500">
            Failed to load file content
          </div>
        );
      case "success":
        if (!blob) return null;
        
        switch (type) {
          case "image":
            return (
              <div className="flex justify-center">
                <img
                  src={URL.createObjectURL(blob)}
                  alt={filename || "Image content"}
                  className="max-w-full max-h-96 object-contain"
                />
              </div>
            );
          case "text":
            return (
              <pre className="whitespace-pre-wrap overflow-auto max-h-96 p-4 text-gray-50 rounded">
                {content}
              </pre>
            );
          case "video":
            return (
              <div className="flex justify-center">
                <ReactPlayer controls={true} url={URL.createObjectURL(blob)} />
              </div>
            );
          default:
            return (
              <div className="flex flex-col items-center justify-center p-8 text-gray-500">
                <File className="w-16 h-16 mb-4" />
                <p>Binary content: ({blob.size} bytes)</p>
              </div>
            );
        }
    }
  };


  return (
    renderContent()
  );
};
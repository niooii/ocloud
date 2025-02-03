import { 
    FolderIcon, 
    FileIcon, 
    FileTextIcon, 
    ImageIcon, 
    FileCodeIcon,
    FileSpreadsheetIcon,
    FileArchiveIcon,
    FileVideoIcon,
    FileAudioIcon,
} from "lucide-react";

// thank you generative AI
export function getFileIcon(filename: string) {
    const extension = filename.split('.').pop()?.toLowerCase();
    
    switch (extension) {
        case 'txt':
        case 'doc':
        case 'docx':
        case 'pdf':
            return <FileTextIcon className="h-4 w-4 text-blue-400" />;
            
        case 'jpg':
        case 'jpeg':
        case 'png':
        case 'gif':
        case 'svg':
            return <ImageIcon className="h-4 w-4 text-green-500" />;
            
        case 'js':
        case 'jsx':
        case 'ts':
        case 'tsx':
        case 'py':
        case 'java':
        case 'cpp':
        case 'html':
        case 'css':
            return <FileCodeIcon className="h-4 w-4 text-purple-500" />;
            
        case 'xlsx':
        case 'csv':
            return <FileSpreadsheetIcon className="h-4 w-4 text-green-600" />;
            
        case 'zip':
        case 'rar':
        case '7z':
        case 'tar':
        case 'gz':
            return <FileArchiveIcon className="h-4 w-4 text-orange-500" />;
            
        case 'mp4':
        case 'mov':
        case 'avi':
            return <FileVideoIcon className="h-4 w-4 text-red-500" />;
            
        case 'mp3':
        case 'wav':
        case 'ogg':
            return <FileAudioIcon className="h-4 w-4 text-yellow-500" />;
            
        default:
            return <FileIcon className="h-4 w-4 text-gray-400" />;
    }
}
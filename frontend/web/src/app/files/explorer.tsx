"use client"

import {
    Table,
    TableBody,
    TableCaption,
    TableCell,
    TableFooter,
    TableHead,
    TableHeader,
    TableRow,
    } from "@/components/ui/table"
import { Path } from "@/lib/api/path"
import { SFile } from "@/lib/api/media"
import { useEffect, useState } from "react"
import { EllipsisVertical, FolderIcon, Slash } from "lucide-react"
import { Card } from "@/components/ui/card"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { getFileIcon } from "./utils"
import { MouseEvent } from 'react'; 
import {
    Dialog,
    DialogContent,
    DialogTitle,
} from "@/components/ui/dialog";
import { MediaApi } from "@/lib/api/media"
import { errorToast, getServerUrl } from "@/lib/include"
import BlobViewer from "./media_viewer"
import MediaViewer from "./media_viewer"
import FileDropArea from "@/components/client/file_dropper"

export function FileExplorer() {
    const [cwd, setCwd] = useState(
        Path.root()
    );
    const [files, setFiles] = useState<SFile[] | null>([]);
    const [viewingMedia, setViewingMedia] = useState(false);
    const [selectedFile, setSelectedFile] = useState<SFile | null>(null);

    useEffect(() => {
        api.listDir(cwd).then(fs => {
            setFiles(fs);
        });
    }, []);

    const api = new MediaApi(getServerUrl()!);

    const updateCwdAndFiles = (newDir: Path) => {
        api.listDir(newDir).then(fs => {
            if (!fs) {
                errorToast(
                    "Something went wrong..",
                    "Check your internet connection and the server's availability.",
                );
            } else {
                // we do this at the same time for visual sync reasons
                setFiles(fs);
                setCwd(newDir);
            }
        });
    }

    const onRowClick = (e: MouseEvent<HTMLTableRowElement>, file: SFile) => {
        console.log(`${file.topLevelName}`);
        if (file.isDir) {
            const newDir = cwd.joinStr(file.topLevelName)!.asDir();
            updateCwdAndFiles(newDir);
        } else {
            setSelectedFile(file);
            setViewingMedia(true);
        }
    };

    return (
        <div className="w-full max-w-7xl">
            <Breadcrumb className="py-2">
                <BreadcrumbList>
                    {cwd.getPathParts().map((part) => (
                        <>
                        <BreadcrumbItem>
                            <BreadcrumbLink>{part}</BreadcrumbLink>
                        </BreadcrumbItem>
                        <BreadcrumbSeparator>
                            <Slash />
                        </BreadcrumbSeparator>
                        </>
                    ))
                    }
                </BreadcrumbList>
            </Breadcrumb>
            <Card className="w-full">
                <Table className="w-full">
                    <TableHeader>
                        <TableRow>
                            <TableHead className="w-[50%]">Name</TableHead>
                            <TableHead className="w-[30%]">Uploaded</TableHead>
                            <TableHead className="w-[33%] text-right">
                            <div className="flex justify-end cursor-pointer">
                                <EllipsisVertical className="h-4 w-4" />
                            </div>
                            </TableHead>
                        </TableRow>
                    </TableHeader>
                    <TableBody className="w-full">
                        {
                        (!cwd.isRoot()) && (
                            <TableRow 
                            key={"prev"} 
                            className="cursor-pointer" 
                            onClick={(e) => {
                                const prev = cwd.clone();
                                prev.pop();
                                updateCwdAndFiles(prev);
                            }}>
                            <TableCell className="w-[50%]">
                                <div className="flex flex-row items-center gap-2 font-medium">
                                <FolderIcon className="h-4 w-4 text-yellow-500" />
                                ..
                                </div>
                            </TableCell>
                            <TableCell className="w-[30%] text-gray-200">
                                
                            </TableCell>
                            <TableCell className="w-[20%] text-right">
                            <div className="flex justify-end">
                                <EllipsisVertical className="h-4 w-4" />
                            </div>
                            </TableCell>
                        </TableRow>
                        )
                        }
                        {
                        files ? (
                            files.map((file) => (
                                <FileDropArea onFileUpload={() => {}}>
                                <TableRow 
                                    key={file.id} 
                                    className="cursor-pointer w-full" 
                                    onClick={(e) => onRowClick(e, file)}>
                                        <TableCell className="w-[50%]">
                                            <div className="flex flex-row items-center gap-2 font-medium">
                                            {file.isDir ? (
                                                <FolderIcon className="h-4 w-4 text-blue-500" />
                                            ) : (
                                                getFileIcon(file.topLevelName)
                                            )}
                                            {file.topLevelName}
                                            </div>
                                        </TableCell>
                                        <TableCell className="w-[30%] text-gray-200">
                                            {file.createdAt.toLocaleString()}
                                        </TableCell>
                                        <TableCell className="w-[20%] text-right">
                                        <div className="flex justify-end">
                                            <EllipsisVertical className="h-4 w-4" />
                                        </div>
                                        </TableCell>
                                    </TableRow>
                                </FileDropArea>
                            ))
                        ) : (<div>bluh</div>)
                        }
                    </TableBody>
                    <Dialog open={viewingMedia} onOpenChange={setViewingMedia}>
                        <DialogContent className="max-w-4xl">
                            <DialogTitle className="flex flex-row items-center gap-4">
                                {getFileIcon(selectedFile?.topLevelName)}
                                {selectedFile?.topLevelName} 
                            </DialogTitle>
                            {viewingMedia && (
                                <MediaViewer 
                                file={selectedFile!} 
                                filename={selectedFile?.topLevelName} 
                                />
                            )}
                        </DialogContent>
                    </Dialog>
                    {/* <TableFooter>
                        <TableRow>
                            <TableCell colSpan={3} className="text-gray-400 text-center">More actions</TableCell>
                        </TableRow>
                    </TableFooter> */}
                </Table>
            </Card>
        </div>
    )
}
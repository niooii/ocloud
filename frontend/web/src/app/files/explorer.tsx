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
import { useEffect, useRef, useState } from "react"
import { EllipsisVertical, FolderIcon, Slash } from "lucide-react"
import { Card } from "@/components/ui/card"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { getFileIcon } from "./utils"
import { MouseEvent, DragEvent } from 'react'; 
import {
    Dialog,
    DialogContent,
    DialogTitle,
} from "@/components/ui/dialog";
import { MediaApi } from "@/lib/api/media"
import { errorToast, getServerUrl } from "@/lib/include"
import {
    Popover,
    PopoverContent,
    PopoverTrigger,
} from "@/components/ui/popover"
import MediaViewer from "./media_viewer"
import FileDropArea from "@/components/client/file_dropper"
import FileUploader from "./upload"
import { Button } from "@/components/ui/button"
import { Input } from "@/components/ui/input"

type SortMethod = "name" | "size" | "datemodified";

export function FileExplorer() {
    const [cwd, setCwd] = useState(
        Path.root()
    );
    const [files, setFiles] = useState<SFile[] | null>([]);
    const [viewingMedia, setViewingMedia] = useState(false);
    const [selectedFile, setSelectedFile] = useState<SFile | null>(null);
    const [sortMethod, setSortMethod] = useState<SortMethod>("name");
    const [sortDirection, setSortDirection] = useState<boolean>(false);
    const [draggedFile, setDraggedFile] = useState<SFile | null>(null);
    const [dragOverFolder, setDragOverFolder] = useState<string | null>(null);

    const setFilesSorted = (fs: SFile[]) => {
        setFiles(sortFileList(fs, sortMethod, sortDirection));
    }

    useEffect(() => {
        updateCwdAndFiles(cwd);
    }, []);

    useEffect(() => {
        setFilesSorted(files!);
    }, [sortMethod, sortDirection]);

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
                setFilesSorted(fs);
                setCwd(newDir);
            }
        });
    }

    const sortFileList = (files: SFile[], method: SortMethod, reverse: boolean = false): SFile[] => {
        const [dirs, sfiles] = files.reduce((acc, file) => {
            acc[file.isDir ? 0 : 1].push(file);
            return acc;
        }, [[] as SFile[], [] as SFile[]]);
        const m = reverse ? -1 : 1;
        
        let sortFunc: (a: SFile, b: SFile) => number;
        switch (sortMethod) {
            case "name": {
                sortFunc = (a: SFile, b: SFile) => {
                    return m * (a.topLevelName.localeCompare(b.topLevelName));
                };
                break;
            }
            case "datemodified": {
                sortFunc = (a: SFile, b: SFile) => {
                    return m * (b.modifiedAt.getTime() - a.modifiedAt.getTime());
                };
                break;
            }
            case "size": {
                // TODO! shit aint implemented
                sortFunc = (a: SFile, b: SFile) => {
                    return m * (b.modifiedAt.getTime() - a.modifiedAt.getTime());
                };
                break;
            }
        }

        dirs.sort(sortFunc);
        sfiles.sort(sortFunc);
        
        const newFiles = [...dirs, ...sfiles];
        return newFiles;
    } 

    const onRowClick = (_e: MouseEvent<HTMLTableRowElement>, file: SFile) => {
        console.log(`${file.topLevelName}`);
        if (file.isDir) {
            const newDir = cwd.joinStr(file.topLevelName)!.asDir();
            updateCwdAndFiles(newDir);
        } else {
            setSelectedFile(file);
            setViewingMedia(true);
        }
    };

    const onFileUpload = (filesToUpload: FileList) => {
        const uploadTo = cwd.clone();
        for (let i = 0; i < filesToUpload.length; i++) {
            const file = filesToUpload[i];
            api.uploadFile(uploadTo, file).then((sfile) => {
                if (!sfile) {
                    console.log("something HAPPENED");
                    return;
                }
                if (uploadTo.equals(cwd)) {
                    const newFiles = [...files!, sfile];
                    setFilesSorted(newFiles);
                }
            });
        }
    }

    const [dirInput, setDirInput] = useState<string | null>(null);
    const [dirInputOpen, setDirInputOpen] = useState<boolean>(false);

    const onDirCreate = () => {
        if (!dirInput) 
            return;
        const targetDir = cwd.clone();
        api.mkDirs(cwd.joinStr(dirInput)!).then((createdDirs) => {
            if (!createdDirs) {
                console.log("SOMETHIGN HAPPENED");
                return;
            }
            if (createdDirs.length == 0) 
                return;
            if (targetDir.equals(cwd)) {
                // Find a newly created folder on the same level as the cwd
                // if it exists update ui
                const target = createdDirs[0];
                if (cwd.getPathParts().length 
                    === target.fullPath.getPathParts().length - 1) {
                    const newFiles = [...files!, target];
                    setFilesSorted(newFiles);
                    setDirInputOpen(false);
                }
            }
        })
    }

    const handleDragStart = (e: DragEvent<HTMLTableRowElement>, file: SFile) => {
        setDraggedFile(file);
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData("text/plain", file.id.toString());
    };

    const handleDragEnd = () => {
        setDraggedFile(null);
        setDragOverFolder(null);
    };

    const handleDragOver = (e: DragEvent<HTMLTableRowElement>, folderName?: string) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        if (folderName) {
            setDragOverFolder(folderName);
        }
    };

    const handleDragLeave = () => {
        setDragOverFolder(null);
    };

    const handleDrop = async (e: DragEvent<HTMLTableRowElement>, targetFolder?: SFile) => {
        e.preventDefault();
        setDragOverFolder(null);
        if (!draggedFile) return;

        // Determine the target path
        let toPath: string | undefined = undefined;
        if (targetFolder && targetFolder.isDir) {
            // Move into the folder (ensure trailing slash)
            toPath = targetFolder.fullPath.asDir().toString() + draggedFile.topLevelName;
            if (draggedFile.isDir) toPath += "/";
        }
        if (targetFolder && !targetFolder.isDir) {
            // Not allowed to drop onto a file
            return;
        }
        // If dropping on ".." (parent directory)
        if (!targetFolder && dragOverFolder === "..") {
            const prev = cwd.clone();
            prev.pop();
            toPath = prev.asDir().toString() + draggedFile.topLevelName;
            if (draggedFile.isDir) toPath += "/";
        }
        if (!toPath) return;

        // Use PATCH and the correct payload
        const fromPath = draggedFile.fullPath.toString();
        const apiUrl = `/files`;
        const serverUrl = getServerUrl();
        try {
            const res = await fetch(`${serverUrl}${apiUrl}`, {
                method: "PATCH",
                headers: {
                    "Content-Type": "application/json",
                    ...(localStorage.getItem('OCLOUD_AUTH') ? { "Authorization": `Bearer ${localStorage.getItem('OCLOUD_AUTH')}` } : {})
                },
                body: JSON.stringify({ from: fromPath, to: toPath })
            });
            if (res.ok) {
                // Remove the file/folder from current view
                const updatedFiles = files!.filter(f => f.id !== draggedFile.id);
                setFilesSorted(updatedFiles);
                // Optionally, refresh the target folder if it's the current cwd
                // Show success message
                console.log(`Moved ${draggedFile.topLevelName} to ${toPath}`);
            } else {
                errorToast(
                    "Move failed",
                    `Failed to move ${draggedFile.topLevelName} to ${toPath}`
                );
            }
        } catch (err) {
            errorToast("Move failed", String(err));
        }
        setDraggedFile(null);
    };

    return (
        <>
        {draggedFile && (
            <div className="fixed top-4 right-4 bg-blue-500 text-white px-4 py-2 rounded-md shadow-lg z-50">
                Dragging: {draggedFile.topLevelName}
            </div>
        )}
        <Popover open={dirInputOpen} onOpenChange={setDirInputOpen}>
            <PopoverTrigger asChild>
                <Button variant="outline">New Folder</Button>
            </PopoverTrigger>
            <PopoverContent className="flex flex-col items-center space-y-4 w-full">
                <Input enterKeyHint="enter" onChange={(e) => {
                    setDirInput(e.target.value);
                }}/>
                <Button variant="outline" onClick={onDirCreate}>
                    Finish.
                </Button>
            </PopoverContent>
        </Popover>
        <div className="w-full max-w-7xl flex-col">
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
                <div className="p-4 text-sm text-muted-foreground border-b">
                    💡 Tip: You can drag files into folders to move them
                </div>
                <Table className="table-fixed">
                    <TableHeader>
                        <TableRow>
                            <TableHead className="w-1/3">Name</TableHead>
                            <TableHead className="w-1/3">Uploaded</TableHead>
                            <TableHead className="w-1/4">Details</TableHead>
                            <TableHead className="text-right">
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
                            className={`cursor-pointer ${
                                dragOverFolder === ".." ? 'bg-blue-100 dark:bg-blue-900' : ''
                            }`}
                            onDragOver={(e) => handleDragOver(e, "..")}
                            onDragLeave={handleDragLeave}
                            onDrop={(e) => handleDrop(e, undefined)}
                            onClick={(e) => {
                                const prev = cwd.clone();
                                prev.pop();
                                updateCwdAndFiles(prev);
                            }}>
                            <TableCell className="">
                                <div className="flex flex-row items-center gap-2 font-medium">
                                <FolderIcon className="h-4 w-4 text-yellow-500" />
                                ..
                                </div>
                            </TableCell>

                            <TableCell className="text-gray-200">
                                
                            </TableCell>

                            <TableCell className="text-muted-foreground">
                                Previous directory
                            </TableCell>

                            <TableCell className="flex justify-end">
                            <div className="">
                                <EllipsisVertical className="h-4 w-4" />
                            </div>
                            </TableCell>
                        </TableRow>
                        )
                        }
                        {
                        files ? (
                            files.map((file) => (
                                <TableRow 
                                key={file.id} 
                                className={`cursor-pointer ${
                                    draggedFile?.id === file.id ? 'opacity-50' : ''
                                } ${
                                    dragOverFolder === file.topLevelName && file.isDir ? 'bg-blue-100 dark:bg-blue-900' : ''
                                }`}
                                draggable={true}
                                onDragStart={(e) => handleDragStart(e, file)}
                                onDragEnd={handleDragEnd}
                                onDragOver={(e) => handleDragOver(e, file.isDir ? file.topLevelName : undefined)}
                                onDragLeave={handleDragLeave}
                                onDrop={(e) => handleDrop(e, file)}
                                onClick={(e) => onRowClick(e, file)}>
                                    <TableCell className="">
                                        <div className="flex flex-row items-center gap-2 font-medium">
                                        {file.isDir ? (
                                            <FolderIcon className="h-4 w-4 text-blue-500" />
                                        ) : (
                                            getFileIcon(file.topLevelName)
                                        )}
                                        {file.topLevelName}
                                        </div>
                                    </TableCell>

                                    <TableCell className="text-gray-200">
                                        {file.createdAt.toLocaleString()}
                                    </TableCell>

                                    <TableCell className="text-muted-foreground">
                                        DETAILS
                                    </TableCell>

                                    <TableCell className="">
                                    <div className="flex justify-end">
                                        <EllipsisVertical className="h-4 w-4" />
                                    </div>
                                    </TableCell>
                                </TableRow>
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
        <FileUploader onChanged={onFileUpload}/>
        </>
    )
}
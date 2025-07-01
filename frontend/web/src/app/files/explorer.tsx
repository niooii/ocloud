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
import { EllipsisVertical, FolderIcon, Slash, Grid, List } from "lucide-react"
import { Card } from "@/components/ui/card"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { getFileIcon } from "./utils"
import { MouseEvent, DragEvent } from 'react'; 
import {
    Dialog,
    DialogContent,
    DialogTitle,
    DialogFooter,
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
import React from "react"

type SortMethod = "name" | "size" | "datemodified";

// Utility to check if a file is an image or video
const isImage = (name: string) => /\.(jpe?g|png|gif|bmp|webp|svg)$/i.test(name);
const isVideo = (name: string) => /\.(mp4|webm|ogg|mov|avi)$/i.test(name);

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
    const [viewMode, setViewMode] = useState<'list' | 'grid'>('grid');

    // Context menu state
    const [contextMenu, setContextMenu] = useState<{ x: number; y: number; visible: boolean }>({ x: 0, y: 0, visible: false });
    const fileInputRef = useRef<HTMLInputElement>(null);

    const [folderDialogOpen, setFolderDialogOpen] = useState(false);
    const [dirInput, setDirInput] = useState<string>("");

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
                    setFolderDialogOpen(false);
                    setDirInput("");
                }
            }
        })
    }

    const handleDragStart = (e: React.DragEvent<any>, file: SFile) => {
        setDraggedFile(file);
        e.dataTransfer.effectAllowed = "move";
        e.dataTransfer.setData("text/plain", file.id.toString());
    };

    const handleDragEnd = () => {
        setDraggedFile(null);
        setDragOverFolder(null);
    };

    const handleDragOver = (e: React.DragEvent<any>, folderName?: string) => {
        e.preventDefault();
        e.dataTransfer.dropEffect = "move";
        if (folderName) {
            setDragOverFolder(folderName);
        }
    };

    const handleDragLeave = (folderName?: string) => {
        // Only clear if leaving the currently highlighted folder
        if (folderName && dragOverFolder === folderName) {
            setTimeout(() => {
                setDragOverFolder((current) => (current === folderName ? null : current));
            }, 50); // Small delay to avoid flicker
        }
    };

    const isDescendantOrSelf = (source: SFile, target: SFile) => {
        // Only relevant for folders
        if (!source.isDir || !target.isDir) return false;
        const sourcePath = source.fullPath.asDir().toString();
        const targetPath = target.fullPath.asDir().toString();
        return targetPath.startsWith(sourcePath);
    };

    const handleDrop = async (e: React.DragEvent<any>, targetFolder?: SFile) => {
        e.preventDefault();
        if (targetFolder && targetFolder.isDir) {
            console.log('Drop on:', targetFolder.topLevelName);
        }
        setDragOverFolder(null);
        if (!draggedFile) return;

        // Prevent dragging a folder into itself or its subfolders
        if (
            draggedFile.isDir &&
            targetFolder &&
            targetFolder.isDir &&
            isDescendantOrSelf(draggedFile, targetFolder)
        ) {
            errorToast("Invalid move", "You cannot move a folder into itself or its subfolders.");
            setDraggedFile(null);
            return;
        }

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

    // Open file picker for upload
    const openFilePicker = () => {
        if (fileInputRef.current) fileInputRef.current.click();
    };

    // Handle right-click on grid background
    const handleGridContextMenu = (e: React.MouseEvent<HTMLDivElement>) => {
        // Only open if not right-clicking on a card
        if ((e.target as HTMLElement).closest('.file-card')) return;
        e.preventDefault();
        setContextMenu({ x: e.clientX, y: e.clientY, visible: true });
    };

    // Close context menu on click elsewhere
    React.useEffect(() => {
        if (!contextMenu.visible) return;
        const close = () => setContextMenu((m) => ({ ...m, visible: false }));
        window.addEventListener('click', close);
        return () => window.removeEventListener('click', close);
    }, [contextMenu.visible]);

    return (
        <>
        <div className="flex items-center justify-between mb-2 gap-4">
            <div className="flex items-center gap-2">
                <Button variant="outline" onClick={() => setFolderDialogOpen(true)}>
                    New Folder
                </Button>
                <Button variant="outline" onClick={openFilePicker}>
                    Upload
                </Button>
            </div>
            <div className="flex gap-2">
                <Button
                    variant={viewMode === 'list' ? 'default' : 'outline'}
                    size="icon"
                    onClick={() => setViewMode('list')}
                    aria-label="List view"
                >
                    <List className="w-5 h-5" />
                </Button>
                <Button
                    variant={viewMode === 'grid' ? 'default' : 'outline'}
                    size="icon"
                    onClick={() => setViewMode('grid')}
                    aria-label="Grid view"
                >
                    <Grid className="w-5 h-5" />
                </Button>
            </div>
        </div>
        <Breadcrumb className="py-2">
            <BreadcrumbList>
                {cwd.getPathParts().map((part, idx, arr) => {
                    const pathUpTo = arr.slice(0, idx + 1).join("/") + "/";
                    return (
                        <React.Fragment key={part + '-' + idx}>
                            <BreadcrumbItem key={"item-" + part + '-' + idx}>
                                <BreadcrumbLink
                                    className="cursor-pointer hover:underline"
                                    onClick={() => {
                                        const newPath = new Path(pathUpTo);
                                        updateCwdAndFiles(newPath);
                                    }}
                                >
                                    {part}
                                </BreadcrumbLink>
                            </BreadcrumbItem>
                            <BreadcrumbSeparator key={"sep-" + part + '-' + idx}>
                                <Slash />
                            </BreadcrumbSeparator>
                        </React.Fragment>
                    );
                })}
            </BreadcrumbList>
        </Breadcrumb>
        <Card className="w-full">
            {viewMode === 'list' ? (
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
                        {(!cwd.isRoot()) && (
                            <TableRow
                                key={"prev"}
                                className={`cursor-pointer ${dragOverFolder === ".." ? 'bg-blue-100 dark:bg-blue-900' : ''}`}
                                onDragOver={(e) => handleDragOver(e, "..")}
                                onDragLeave={() => handleDragLeave("..")}
                                onDrop={(e) => handleDrop(e, undefined)}
                                onClick={(e) => {
                                    const prev = cwd.clone();
                                    prev.pop();
                                    updateCwdAndFiles(prev);
                                }}
                            >
                                <TableCell className="">
                                    <div className="flex flex-row items-center gap-2 font-medium">
                                        <FolderIcon className="h-4 w-4 text-yellow-500" />
                                        ..
                                    </div>
                                </TableCell>
                                <TableCell className="text-gray-200"></TableCell>
                                <TableCell className="text-muted-foreground">Previous directory</TableCell>
                                <TableCell className="flex justify-end">
                                    <div className="">
                                        <EllipsisVertical className="h-4 w-4" />
                                    </div>
                                </TableCell>
                            </TableRow>
                        )}
                        {files ? (
                            files.map((file) => (
                                <TableRow
                                    key={file.id}
                                    className={`cursor-pointer ${
                                        draggedFile?.id === file.id ? 'ring-2 ring-blue-500 bg-blue-50 dark:bg-blue-900/60' : ''
                                    } ${
                                        dragOverFolder === file.topLevelName && file.isDir ? 'bg-blue-100 dark:bg-blue-900' : ''
                                    }`}
                                    draggable={true}
                                    onDragStart={(e) => {
                                        handleDragStart(e, file);
                                        // Custom drag image
                                        const crt = document.createElement('div');
                                        crt.style.position = 'absolute';
                                        crt.style.top = '-1000px';
                                        crt.style.left = '-1000px';
                                        crt.style.padding = '8px 16px';
                                        crt.style.background = '#2563eb';
                                        crt.style.color = 'white';
                                        crt.style.fontWeight = 'bold';
                                        crt.style.borderRadius = '6px';
                                        crt.style.boxShadow = '0 2px 8px rgba(0,0,0,0.2)';
                                        crt.innerText = file.topLevelName + (file.isDir ? '/' : '');
                                        document.body.appendChild(crt);
                                        e.dataTransfer.setDragImage(crt, 0, 0);
                                        setTimeout(() => document.body.removeChild(crt), 0);
                                    }}
                                    onDragEnd={handleDragEnd}
                                    onDragOver={(e) => handleDragOver(e, file.isDir ? file.topLevelName : undefined)}
                                    onDragLeave={() => handleDragLeave(file.isDir ? file.topLevelName : undefined)}
                                    onDrop={(e) => handleDrop(e, file)}
                                    onClick={(e) => onRowClick(e, file)}
                                >
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
                                    <TableCell className="text-gray-200">{file.createdAt.toLocaleString()}</TableCell>
                                    <TableCell className="text-muted-foreground">DETAILS</TableCell>
                                    <TableCell className="">
                                        <div className="flex justify-end">
                                            <EllipsisVertical className="h-4 w-4" />
                                        </div>
                                    </TableCell>
                                </TableRow>
                            ))
                        ) : (<div>bluh</div>)}
                    </TableBody>
                </Table>
            ) : (
                // GRID VIEW
                <div className="p-4 relative" onContextMenu={handleGridContextMenu}>
                    <div className="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-6 gap-6">
                        {(!cwd.isRoot()) && (
                            <div
                                key="prev"
                                className={`flex flex-col items-center justify-center cursor-pointer rounded-lg border border-dashed border-gray-300 p-6 transition
                                    ${dragOverFolder === ".." ? 'bg-blue-100 dark:bg-blue-900' : 'bg-white dark:bg-zinc-900'}`}
                                onClick={() => {
                                    const prev = cwd.clone();
                                    prev.pop();
                                    updateCwdAndFiles(prev);
                                }}
                                onDragOver={(e) => handleDragOver(e, "..")}
                                onDragLeave={() => handleDragLeave("..")}
                                onDrop={(e) => handleDrop(e, undefined)}
                            >
                                <FolderIcon className="h-8 w-8 text-yellow-500 mb-2" />
                                <span className="text-xs text-muted-foreground">..</span>
                            </div>
                        )}
                        {files && files.length > 0 ? files.map((file) => (
                            <div
                                key={file.id}
                                className={`file-card group relative flex flex-col items-center justify-center p-4 rounded-xl shadow-sm border border-gray-200 dark:border-zinc-800 cursor-pointer transition hover:shadow-lg
                                    ${draggedFile?.id === file.id ? 'ring-2 ring-blue-500' : ''}
                                    ${dragOverFolder === file.topLevelName && file.isDir ? 'bg-blue-100 dark:bg-blue-900' : 'bg-white dark:bg-zinc-900'}
                                `}
                                draggable={true}
                                onDragStart={(e) => {
                                    handleDragStart(e, file);
                                    // Custom drag image
                                    const crt = document.createElement('div');
                                    crt.style.position = 'absolute';
                                    crt.style.top = '-1000px';
                                    crt.style.left = '-1000px';
                                    crt.style.padding = '8px 16px';
                                    crt.style.background = '#2563eb';
                                    crt.style.color = 'white';
                                    crt.style.fontWeight = 'bold';
                                    crt.style.borderRadius = '6px';
                                    crt.style.boxShadow = '0 2px 8px rgba(0,0,0,0.2)';
                                    crt.innerText = file.topLevelName + (file.isDir ? '/' : '');
                                    document.body.appendChild(crt);
                                    e.dataTransfer.setDragImage(crt, 0, 0);
                                    setTimeout(() => document.body.removeChild(crt), 0);
                                }}
                                onDragEnd={handleDragEnd}
                                onDragOver={file.isDir ? (e) => { handleDragOver(e, file.topLevelName); } : undefined}
                                onDragLeave={file.isDir ? (e) => { handleDragLeave(file.topLevelName); } : undefined}
                                onDrop={file.isDir ? (e) => handleDrop(e, file) : undefined}
                                onClick={() => {
                                    if (file.isDir) {
                                        const newDir = cwd.joinStr(file.topLevelName)!.asDir();
                                        updateCwdAndFiles(newDir);
                                    } else {
                                        setSelectedFile(file);
                                        setViewingMedia(true);
                                    }
                                }}
                            >
                                <div className="w-20 h-20 flex items-center justify-center rounded-lg bg-zinc-100 dark:bg-zinc-800 mb-2 overflow-hidden">
                                    {isImage(file.topLevelName) ? (
                                        <img
                                            src={getServerUrl() + '/files/' + file.fullPath.toString()}
                                            alt={file.topLevelName}
                                            className="object-cover w-full h-full"
                                        />
                                    ) : isVideo(file.topLevelName) ? (
                                        <video
                                            src={getServerUrl() + '/files/' + file.fullPath.toString()}
                                            className="object-cover w-full h-full"
                                            controls={false}
                                            muted
                                        />
                                    ) : file.isDir ? (
                                        <FolderIcon className="w-10 h-10 text-blue-500" />
                                    ) : (
                                        <span className="w-10 h-10 flex items-center justify-center">{getFileIcon(file.topLevelName)}</span>
                                    )}
                                </div>
                                <span className="text-sm font-medium text-center truncate w-24">
                                    {file.topLevelName}
                                </span>
                                <span className="text-xs text-muted-foreground mt-1">
                                    {file.isDir ? 'Folder' : isImage(file.topLevelName) ? 'Image' : isVideo(file.topLevelName) ? 'Video' : 'File'}
                                </span>
                                <div className="absolute top-2 right-2 opacity-0 group-hover:opacity-100 transition">
                                    <EllipsisVertical className="h-4 w-4 text-gray-400" />
                                </div>
                            </div>
                        )) : (
                            <div className="col-span-full text-center text-muted-foreground py-8">No files or folders</div>
                        )}
                    </div>
                    {/* Context Menu */}
                    {contextMenu.visible && (
                        <div
                            className="fixed z-50 bg-white dark:bg-zinc-900 rounded-lg shadow-lg border border-gray-200 dark:border-zinc-800 py-2 px-2 min-w-[180px]"
                            style={{ left: contextMenu.x, top: contextMenu.y }}
                        >
                            <button
                                className="w-full text-left px-3 py-2 rounded hover:bg-blue-100 dark:hover:bg-blue-800 transition"
                                onClick={() => {
                                    setFolderDialogOpen(true);
                                    setContextMenu((m) => ({ ...m, visible: false }));
                                }}
                            >
                                Create a folder
                            </button>
                            <button
                                className="w-full text-left px-3 py-2 rounded hover:bg-blue-100 dark:hover:bg-blue-800 transition"
                                onClick={() => {
                                    openFilePicker();
                                    setContextMenu((m) => ({ ...m, visible: false }));
                                }}
                            >
                                Upload files
                            </button>
                        </div>
                    )}
                </div>
            )}
        </Card>
        {/* Media viewer dialog always rendered so it works in both views */}
        <Dialog open={viewingMedia} onOpenChange={setViewingMedia}>
            <DialogContent className="max-w-4xl">
                <DialogTitle className="flex flex-row items-center gap-4">
                    {getFileIcon(selectedFile?.topLevelName)}
                    {selectedFile?.topLevelName}
                </DialogTitle>
                {viewingMedia && (
                    <MediaViewer file={selectedFile!} filename={selectedFile?.topLevelName} />
                )}
            </DialogContent>
        </Dialog>
        <FileUploader onChanged={onFileUpload} />
        {/* Folder creation dialog */}
        <Dialog open={folderDialogOpen} onOpenChange={setFolderDialogOpen}>
            <DialogContent>
                <DialogTitle>Create a new folder</DialogTitle>
                <Input
                    autoFocus
                    placeholder="Folder name"
                    value={dirInput}
                    onChange={e => setDirInput(e.target.value)}
                    onKeyDown={e => {
                        if (e.key === 'Enter') {
                            onDirCreate();
                        }
                    }}
                />
                <DialogFooter>
                    <Button variant="outline" onClick={() => setFolderDialogOpen(false)}>
                        Cancel
                    </Button>
                    <Button
                        onClick={onDirCreate}
                        disabled={!dirInput.trim()}
                    >
                        Create
                    </Button>
                </DialogFooter>
            </DialogContent>
        </Dialog>
        {/* Hidden file input for upload (always present) */}
        <input
            ref={fileInputRef}
            type="file"
            style={{ display: 'none' }}
            onChange={e => {
                if (e.target.files && e.target.files.length > 0) {
                    onFileUpload(e.target.files);
                }
            }}
        />
        </>
    )
}
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
import { SFile } from "@/lib/api/types"
import { useEffect, useState } from "react"
import { EllipsisVertical, FolderIcon, Slash } from "lucide-react"
import { Card } from "@/components/ui/card"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { getFileIcon } from "./utils"
import { MouseEvent } from 'react'; 

import { MediaApi } from "@/lib/api/media"
import { getServerUrl } from "@/lib/include"

export function FileExplorer() {
    const [cwd, setCwd] = useState(
        Path.root()
    );
    const [files, setFiles] = useState<SFile[]>([]);

    useEffect(() => {
        api.listDir(cwd).then(fs => {
            setFiles(fs);
        });
    }, [cwd]);

    const api = new MediaApi(getServerUrl()!);

    const onRowClick = (e: MouseEvent<HTMLTableRowElement>, file: SFile) => {
        console.log(`${file.topLevelName}`);
        if (file.isDir) {
            const newDir = cwd.joinStr(file.topLevelName)!.asDir();
            setCwd(newDir);
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
                    {/* <TableCaption>A list of your recent invoices.</TableCaption> */}
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
                                    setCwd(prev);
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
                            files.map((file) => (
                                <TableRow 
                                key={file.id} 
                                className="cursor-pointer" 
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
                            ))
                        }
                    </TableBody>
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
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
import { useState } from "react"
import { EllipsisVertical, FolderIcon, Slash } from "lucide-react"
import { Card } from "@/components/ui/card"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { getFileIcon } from "./utils"
import { MouseEvent } from 'react'; 

import {
    ContextMenu,
    ContextMenuContent,
    ContextMenuItem,
    ContextMenuTrigger,
} from "@/components/ui/context-menu"

const testFiles: SFile[] = [
    {
        id: 1,
        isDir: true,
        fullPath: new Path("root/documents"),
        createdAt: new Date("2025-02-03T01:02:53.539781"),
        modifiedAt: new Date("2025-02-03T01:02:53.539781"),
        topLevelName: "documents"
    },
    {
        id: 2,
        isDir: false,
        fullPath: new Path("root/documents/resume.pdf"),
        createdAt: new Date("2025-02-03T01:03:12.123456"),
        modifiedAt: new Date("2025-02-03T01:15:22.987654"),
        topLevelName: "resume.pdf"
    },
    {
        id: 3,
        isDir: true,
        fullPath: new Path("root/documents/receipts"),
        createdAt: new Date("2025-02-03T02:15:00.111222"),
        modifiedAt: new Date("2025-02-03T02:15:00.111222"),
        topLevelName: "receipts"
    },
    {
        id: 4,
        isDir: false,
        fullPath: new Path("root/documents/receipts/jan2025.pdf"),
        createdAt: new Date("2025-02-03T02:16:33.444555"),
        modifiedAt: new Date("2025-02-03T02:16:33.444555"),
        topLevelName: "jan2025.pdf"
    },
    {
        id: 5,
        isDir: true,
        fullPath: new Path("root/images"),
        createdAt: new Date("2025-02-03T03:00:00.000000"),
        modifiedAt: new Date("2025-02-03T03:00:00.000000"),
        topLevelName: "images"
    },
    {
        id: 6,
        isDir: false,
        fullPath: new Path("root/images/profile.jpg"),
        createdAt: new Date("2025-02-03T03:01:15.666777"),
        modifiedAt: new Date("2025-02-03T03:30:45.888999"),
        topLevelName: "profile.jpg"
    },
    {
        id: 7,
        isDir: false,
        fullPath: new Path("root/images/banner.png"),
        createdAt: new Date("2025-02-03T03:02:30.123456"),
        modifiedAt: new Date("2025-02-03T03:02:30.123456"),
        topLevelName: "banner.png"
    }
]
   
export function FileExplorer() {
    const [cwd, setCwd] = useState(
        Path.root()
    );

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
                        {testFiles.map((file) => (
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
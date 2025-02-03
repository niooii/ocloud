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
import { FolderIcon, Slash } from "lucide-react"
import { Card } from "@/components/ui/card"
import { Breadcrumb, BreadcrumbItem, BreadcrumbLink, BreadcrumbList, BreadcrumbSeparator } from "@/components/ui/breadcrumb"
import { getFileIcon } from "./utils"
import { MouseEvent } from 'react'; 

// is_dir: boolean,
// full_path: String,
// created_at: BigInt,
// modified_at: BigInt,
// // Either the name of the directory or the file
// top_level_name: String
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
        Path.root().joinStr("dir/somedir/test/")!
    );

    const onRowClick = (e: MouseEvent<HTMLTableRowElement>, file: SFile) => {
        console.log(`${file.topLevelName}`);
        if (file.isDir) {
            console.log(`${cwd}`);
            setCwd(cwd.joinStr(file.topLevelName)!);
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
                                <TableHead className="w-full">Name</TableHead>
                                <TableHead>Uploaded</TableHead>
                                <TableHead className="text-right">Size</TableHead>
                            </TableRow>
                        </TableHeader>
                    <TableBody>
                        {testFiles.map((file) => (
                            <TableRow 
                            key={file.id} 
                            className="cursor-pointer" 
                            onClick={(e) => onRowClick(e, file)}>
                                <TableCell className="flex flex-row items-center gap-2 font-medium">
                                    {
                                        file.isDir ? (
                                            <FolderIcon className="h-4 w-4 text-blue-500" />
                                        ) : (
                                            getFileIcon(file.topLevelName)
                                        )
                                    }
                                    {file.topLevelName}
                                </TableCell>
                                <TableCell>{file.createdAt.toLocaleDateString()}</TableCell>
                                <TableCell className="text-right">NOT YET</TableCell>
                            </TableRow>
                        ))
                        }
                    </TableBody>
                    <TableFooter>
                        <TableRow>
                            <TableCell colSpan={2}>Total (i stole this)</TableCell>
                            <TableCell className="text-right">$2,500.00</TableCell>
                        </TableRow>
                    </TableFooter>
                </Table>
            </Card>
        </div>
    )
}
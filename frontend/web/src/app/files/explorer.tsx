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

// is_dir: boolean,
// full_path: String,
// created_at: BigInt,
// modified_at: BigInt,
// // Either the name of the directory or the file
// top_level_name: String
const testFiles: SFile[] = [
    {
        is_dir: true,
        full_path: new Path("root/documents"),
        created_at: new Date("2025-02-03T01:02:53.539781"),
        modified_at: new Date("2025-02-03T01:02:53.539781"),
        top_level_name: "documents"
    },
    {
        is_dir: false,
        full_path: new Path("root/documents/resume.pdf"),
        created_at: new Date("2025-02-03T01:03:12.123456"),
        modified_at: new Date("2025-02-03T01:15:22.987654"),
        top_level_name: "resume.pdf"
    },
    {
        is_dir: true,
        full_path: new Path("root/documents/receipts"),
        created_at: new Date("2025-02-03T02:15:00.111222"),
        modified_at: new Date("2025-02-03T02:15:00.111222"),
        top_level_name: "receipts"
    },
    {
        is_dir: false,
        full_path: new Path("root/documents/receipts/jan2025.pdf"),
        created_at: new Date("2025-02-03T02:16:33.444555"),
        modified_at: new Date("2025-02-03T02:16:33.444555"),
        top_level_name: "jan2025.pdf"
    },
    {
        is_dir: true,
        full_path: new Path("root/images"),
        created_at: new Date("2025-02-03T03:00:00.000000"),
        modified_at: new Date("2025-02-03T03:00:00.000000"),
        top_level_name: "images"
    },
    {
        is_dir: false,
        full_path: new Path("root/images/profile.jpg"),
        created_at: new Date("2025-02-03T03:01:15.666777"),
        modified_at: new Date("2025-02-03T03:30:45.888999"),
        top_level_name: "profile.jpg"
    },
    {
        is_dir: false,
        full_path: new Path("root/images/banner.png"),
        created_at: new Date("2025-02-03T03:02:30.123456"),
        modified_at: new Date("2025-02-03T03:02:30.123456"),
        top_level_name: "banner.png"
    }
   ]
   
export function FileExplorer() {
    return (
        <Table className="w-full">
            <TableCaption>A list of your recent invoices.</TableCaption>
                <TableHeader>
                    <TableRow>
                        <TableHead className="w-full">File</TableHead>
                        <TableHead>Upload Date</TableHead>
                        <TableHead className="text-right">Size</TableHead>
                    </TableRow>
                </TableHeader>
            <TableBody>
                {testFiles.map((file) => (
                    <TableRow key={file.top_level_name} className="cursor-pointer">
                        <TableCell className="font-medium">{file.top_level_name}</TableCell>
                        <TableCell>{file.created_at.toLocaleDateString()}</TableCell>
                        <TableCell className="text-right">NOT YET</TableCell>
                    </TableRow>
                ))}
            </TableBody>
            <TableFooter>
                <TableRow>
                    <TableCell colSpan={2}>Total (i stole this)</TableCell>
                    <TableCell className="text-right">$2,500.00</TableCell>
                </TableRow>
            </TableFooter>
        </Table>
    )
}
"use client"

import Link from "next/link"
import Image from "next/image";
import { Menu } from "lucide-react";

const Navbar = () => {
    return (
        <nav className="z-[20] flex items-center justify-between h-20 mx-auto px-4 bg-background/80 backdrop-blur-sm border-b border-gray-300">
            <h1 className="font-bold text-3xl w-full">
                <Link href="/">oCloud</Link>
            </h1>
            <div className="flex">
                <Link href="/home" className="p-4">Home</Link>
                <Link href="/#about" className="p-4">About</Link>
                <Link href="https://github.com/niooii/ocloud" className="p-4">Documentation</Link>
                <Link href="/#other" className="p-4">Other</Link>
            </div>
        </nav>
    );
};

export default Navbar;
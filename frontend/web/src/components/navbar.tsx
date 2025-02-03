"use client"

import Link from "next/link"
import Image from "next/image";

const Navbar = () => {
    return (
        <nav className="top-0 z-[20] mx-auto w-full flex items-center justify-between bg-background/80 backdrop-blur-sm border-b border-gray-500 p-5">
            <h1 className="font-bold text-3xl w-full">
                oCloud
            </h1>
            <div className="flex">
                <Link href="/home" className="p-4">Home</Link>
                <Link href="/about" className="p-4">About</Link>
                <Link href="/documentation" className="p-4">Documentation</Link>
                <Link href="/other" className="p-4">Other</Link>
            </div>
        </nav>
    );
};

export default Navbar;
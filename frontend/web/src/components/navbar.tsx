"use client"

import Link from "next/link"
import Image from "next/image";
import { Menu, X} from "lucide-react";
import { useState } from "react";

const Navbar = () => {
    const [menuOpen, setMenuOpen] = useState(false);

    const handleMenu = () => {
        setMenuOpen(!menuOpen);
    };

    return (
        <nav className="z-[20] flex items-center justify-between h-20 mx-auto px-4 bg-background/80 backdrop-blur-sm border-b border-gray-300 text-base">
            <h1 className="font-bold text-3xl w-full">
                <Link href="/">oCloud</Link>
            </h1>
            <div>
            <ul className="flex hidden sm:flex">
            <Link href="/home">
                <li className="px-4 py-1 border-b-2 border-transparent hover:border-gray-300 transition-all duration-200">Home</li>
            </Link>
            <Link href="/#about">
                <li className="px-4 py-1 border-b-2 border-transparent hover:border-gray-300 transition-all duration-200">About</li>
            </Link>
            <Link href="https://github.com/niooii/ocloud">
                <li className="px-4 py-1 border-b-2 border-transparent hover:border-gray-300 transition-all duration-200">Documentation</li>
            </Link>
            <Link href="/#other">
                <li className="px-4 py-1 border-b-2 border-transparent hover:border-gray-300 transition-all duration-200">Other</li>
            </Link>
            </ul>
            </div>
            <div onClick={handleMenu} className="sm:hidden cursor-pointer pl-24">
                <div>
                    {menuOpen ? <X size={25}/> : <Menu size={25}/>}
                </div>
            </div>

            <div className={
                menuOpen ? "fixed z-20 left-0 top-0 w-[40%] sm:hidden h-screen bg-[#1e1e1e] p-10 ease-out duration-500"
                : "fixed left-[-100%] h-screen top-0 p-10 ease-in duration-500"
            }
            >
                <div className="flex-col">
                <ul>
                <Link href="/home">
                    <li onClick={() => setMenuOpen(false)} className="py-4 border-transparent hover:border-b-2 hover:border-gray-300 transition-all duration-200">Home</li>
                </Link>
                <Link href="/#about">
                    <li onClick={() => setMenuOpen(false)} className="py-4 border-transparent hover:border-b-2 hover:border-gray-300 transition-all duration-200">About</li>
                </Link>
                <Link href="https://github.com/niooii/ocloud">
                    <li onClick={() => setMenuOpen(false)} className="py-4 border-transparent hover:border-b-2 hover:border-gray-300 transition-all duration-200">Documentation</li>
                </Link>
                <Link href="/#other">
                    <li onClick={() => setMenuOpen(false)} className="py-4 border-transparent hover:border-b-2 hover:border-gray-300 transition-all duration-200">Other</li>
                </Link>
                </ul>
                </div>
            </div>

        </nav>
    );
};

export default Navbar;
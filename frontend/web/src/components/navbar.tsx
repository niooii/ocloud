"use client"

import Link from "next/link"
import Image from "next/image";
import {
    NavigationMenu,
    NavigationMenuContent,
    NavigationMenuIndicator,
    NavigationMenuItem,
    NavigationMenuLink,
    NavigationMenuList,
    NavigationMenuTrigger,
    NavigationMenuViewport,
    navigationMenuTriggerStyle
  } from "@/components/ui/navigation-menu"
import { Card } from "./ui/card";

export default function Navbar() {
    return (
        <div className="fixed top-0 w-full border-b bg-background/80 backdrop-blur-sm z-50">
            <div className="container flex h-14 items-center">
                <nav className="flex items-center space-x-6 text-sm font-medium p-4">
                    <Image
                        src="/onion.svg"
                        alt="onion"
                        width={50}
                        height={50}
                        priority
                        className="brightness-0 invert"
                    />
                    <NavigationMenu>
                        <NavigationMenuList>
                            <NavigationMenuItem>
                                <NavigationMenuTrigger>SOME MENU</NavigationMenuTrigger>
                                <NavigationMenuContent>
                                    <NavigationMenuLink>
                                        <Card className="p-4">
                                            Type shit..
                                        </Card>
                                    </NavigationMenuLink>
                                </NavigationMenuContent>
                                <NavigationMenuLink href="https://github.com/niooii/ocloud" className={navigationMenuTriggerStyle()}>
                                    Documentation
                                </NavigationMenuLink>
                            </NavigationMenuItem>
                        </NavigationMenuList>
                    </NavigationMenu>
                </nav>
            </div>
        </div>
    )
}
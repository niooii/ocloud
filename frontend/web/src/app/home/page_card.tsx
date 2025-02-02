"use client"

import {
    Card,
    CardContent,
    CardDescription,
    CardFooter,
    CardHeader,
    CardTitle,
} from "@/components/ui/card"

import Image from "next/image";

import { useRouter } from "next/navigation"

interface CardProps {
    title: string;
    description?: string; 
    href: string; 
}

export default function PageCard({ title, description, href }: CardProps) {
    const router = useRouter();
    
    return (
        <div className="p-4">
            <Card 
                className="opacity-80 w-[350px] p-4 cursor-pointer hover:opacity-90 transition-all"
                onClick={() => router.push(href)}
            >
                <CardContent className="flex flex-col items-center justify-center min-h-48">
                    {/* TODO! make this a prop too */}
                    <Image
                        src="/folder.svg"
                        alt="onion"
                        width={150}
                        height={150}
                        priority
                        className="invert"
                    />
                    <h3 className="text-2xl font-semibold text-center 5">
                        {title}
                    </h3>
                </CardContent>
                    {/* <CardDescription className="text-lg text-left">
                        {description}
                    </CardDescription> */}
            </Card>
        </div>
    )
}
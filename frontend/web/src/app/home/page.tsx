import PageCard from "./page_card";
import Navbar from "@/components/navbar";
import { AppSidebar } from "@/components/sidebar"
import { SidebarProvider } from "@/components/ui/sidebar";
import { Sidebar } from "lucide-react";

export default function Home() {
    return (
        <main className="font-[family-name:var(--font-geist-sans)]">
            <Navbar/>
            <div className="flex flex-col min-h-screen justify-center items-center">
                <h1 className="text-4xl font-semibold">Welcome!</h1>
                <div>
                    <PageCard title="Manage your files" href="/files"/>
                </div>
            </div>
        </main>
    );
}
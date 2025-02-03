import PageCard from "./page_card";
import Navbar from "@/components/navbar";
import { AppSidebar } from "@/components/sidebar"
import { SidebarProvider } from "@/components/ui/sidebar";
import { Sidebar } from "lucide-react";

export default function Home() {
    return (
        <main className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
            <Navbar/>
            <SidebarProvider>
                <AppSidebar/>
            </SidebarProvider>
            <h1 className="text-4xl font-semibold">Welcome!</h1>
            <div>
                <PageCard title="Manage your files" href="/files"/>
            </div>
        </main>
    );
}
import Navbar from "@/components/navbar";
import { Input } from "@/components/ui/input";
import FileUploader from "./upload";
import { FileExplorer } from "./explorer";

export default function Files() {
    return (
        <div className="font-[family-name:var(--font-geist-sans)]">
            <Navbar/>
            <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20">
            <main className="flex flex-col gap-4 row-start-2 items-center -mt-20 w-full">
                <FileExplorer/>
                UPLOAD
                <FileUploader/>
            </main>
            </div>
        </div>
    );
}
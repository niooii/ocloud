import Navbar from "@/components/navbar";

export default function Files() {
    const title = "Hello";

    return (
        <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-8 pb-20 gap-16 sm:p-20 font-[family-name:var(--font-geist-sans)]">
            <Navbar/>
            <main className="flex flex-col gap-4 row-start-2 items-center -mt-20">
                FILES
            </main>
            <footer className="row-start-3 flex gap-6 flex-wrap items-center justify-center">

            </footer>
        </div>
    );
}
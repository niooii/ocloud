import Image from "next/image";
import Link from "next/link";
import LandingForm from "../components/client/landing_page_form";
import { Path } from "@/lib/api/path";
import Navbar from "../components/navbar";

export default function Landing() {
  // testing path stuff
  let path = new Path("root/test////TEST2///testshit///")
  console.log(path.name())

  return (
    <div className="font-[family-name:var(--font-geist-sans)]">
      <Navbar/>
      <div className="grid grid-rows-[20px_1fr_20px] items-center justify-items-center min-h-screen p-4 pb-20 gap-16 sm:p-5">
      <main className="flex flex-col gap-4 row-start-2 items-center -mt-20">
        {/* <text className="text-7xl font-bold">oCloud</text> */}
        <Image
          src="/onion_logo.svg"
          alt="onion"
          width={500}
          height={180}
          priority
        />
        <LandingForm/>
      </main>
      <footer className="row-start-3 flex gap-6 flex-wrap items-center justify-center">
        <Link
          className="flex items-center gap-2 hover:underline hover:underline-offset-4"
          href="https://github.com/niooii/ocloud"
          target="_blank"
          rel="noopener noreferrer"
        >
          <Image
            aria-hidden
            src="/github.svg"
            alt="Git icon"
            width={30}
            height={30}
          />
          Github
        </Link>
      </footer>
      </div>
    </div>
  );
}

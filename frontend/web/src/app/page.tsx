import Image from "next/image";
import Link from "next/link";
import Landing from "../components/landing"
import LandingForm from "../components/client/landing_page_form";
import { Path } from "@/lib/api/path";
import Navbar from "../components/navbar";

export default function App() {
  // testing path stuff
  let path = new Path("root/test////TEST2///testshit///")
  console.log(path.name())

  return (
    <div className="font-[family-name:var(--font-geist-sans)]">
      <Navbar/>
      <Landing/>
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
  );
}

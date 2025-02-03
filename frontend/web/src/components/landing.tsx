"use client"
import Image from "next/image";
import Link from "next/link";
import LandingForm from "../components/client/landing_page_form";

const Landing = () => {
    return (
        <div className="grid-cols-1 gap-4 row-start-2 items-center justify-items-center">
            <Image
            src="/onion_logo.svg"
            alt="onion"
            width={500}
            height={180}
            priority
            />

            <LandingForm/>
        </div>
        
    );
};

export default Landing;
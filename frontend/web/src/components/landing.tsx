"use client"
import Image from "next/image";
import Link from "next/link";
import LandingForm from "../components/client/landing_page_form";

const Landing = () => {
    return (
        <div className="flex flex-col min-h-screen justify-center items-center">
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
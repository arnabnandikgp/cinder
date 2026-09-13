import type { Metadata } from "next";
import { Inter, JetBrains_Mono } from "next/font/google";
import "./globals.css";

const inter = Inter({
  subsets: ["latin"],
  variable: "--font-sans",
  display: "swap",
});

const jetbrainsMono = JetBrains_Mono({
  subsets: ["latin"],
  variable: "--font-mono",
  display: "swap",
});

export const metadata: Metadata = {
  title: "Cinder — Private Perpetuals. Phoenix Liquidity.",
  description:
    "Cinder is an experimental private perp prime broker for Phoenix on Solana. Confidential user accounting in MagicBlock PER with aggregate venue execution.",
  keywords: ["Solana", "Phoenix DEX", "MagicBlock", "Perpetuals", "Privacy", "Prime Broker", "DeFi"],
  openGraph: {
    title: "Cinder — Private Perpetuals. Phoenix Liquidity.",
    description: "Confidential user accounting in MagicBlock PER with aggregate venue execution on Phoenix.",
    siteName: "Cinder Protocol",
    type: "website",
  },
};

export default function RootLayout({ children }: Readonly<{ children: React.ReactNode }>) {
  return (
    <html lang="en" className={`${inter.variable} ${jetbrainsMono.variable}`}>
      <body>{children}</body>
    </html>
  );
}

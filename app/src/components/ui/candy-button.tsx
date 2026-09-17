import React from "react";
import { cn } from "@/lib/utils";

export interface CandyButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  tone?: "info" | "danger" | "neutral";
  size?: "default" | "compact" | "tiny";
}

const TONES: Record<string, string> = {
  info: cn(
    "border-[#54A1FD] bg-[radial-gradient(95%_60%_at_50%_75%,#005FD6_0%,#209BFF_100%)]",
    "shadow-[0px_4px_48px_-12px_#1187FF,inset_0px_1px_8px_-4px_#FFFFFF]"
  ),
  danger: cn(
    "border-[#F0787B] bg-[radial-gradient(95%_60%_at_50%_75%,#8F1D1D_0%,#EA5457_100%)]",
    "shadow-[0px_4px_48px_-12px_#F0787B,inset_0px_1px_8px_-4px_#FFFFFF]"
  ),
  neutral: cn(
    "border-[#8A93A6] bg-[radial-gradient(95%_60%_at_50%_75%,#3A4150_0%,#697487_100%)]",
    "shadow-[0px_4px_48px_-12px_#8A93A6,inset_0px_1px_8px_-4px_#FFFFFF]"
  ),
};

const SIZES: Record<string, string> = {
  default: "px-9 py-3 rounded-xl text-base leading-[22px]",
  compact: "px-3 py-1 rounded-lg text-[12px] leading-[18px] gap-1.5",
  tiny: "px-2 py-0.5 rounded-md text-[11px] leading-[16px] gap-1",
};

export function CandyButton({
  className,
  tone = "info",
  size = "compact",
  children,
  ...props
}: CandyButtonProps) {
  return (
    <button
      className={cn(
        "candy relative inline-flex items-center justify-center",
        "text-white font-semibold tracking-[0.02em]",
        "cursor-pointer border transition-all duration-200 ease-out",
        "after:absolute after:top-[1px] after:right-[10%] after:w-[60%] after:h-[1px]",
        "after:bg-gradient-to-r after:from-transparent after:via-white/50 after:to-transparent",
        "hover:brightness-110 active:scale-95 active:rotate-1",
        "disabled:opacity-50 disabled:pointer-events-none",
        SIZES[size],
        TONES[tone],
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}

export default CandyButton;

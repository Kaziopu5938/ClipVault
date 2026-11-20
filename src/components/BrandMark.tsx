import { cn } from "../lib/cn";

export function BrandMark({ size = 25, className }: { size?: number; className?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 128 128"
      fill="none"
      className={cn("brand-mark", className)}
      aria-hidden="true"
    >
      <rect x="2" y="2" width="124" height="124" rx="30" fill="#17181C" />
      <rect x="2.75" y="2.75" width="122.5" height="122.5" rx="29.25" stroke="#3A3D46" strokeWidth="1.5" />
      <path
        d="M45 31.5h38a7 7 0 0 1 7 7v59a7 7 0 0 1-7 7H45a7 7 0 0 1-7-7v-59a7 7 0 0 1 7-7Z"
        fill="#EDE7DA"
      />
      <path
        d="M54.5 24h19a5 5 0 0 1 5 5v11h-29V29a5 5 0 0 1 5-5Z"
        fill="#C9A96E"
      />
      <path
        d="M49.5 55.5 64 89l14.5-33.5"
        stroke="#17181C"
        strokeWidth="9.5"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
}

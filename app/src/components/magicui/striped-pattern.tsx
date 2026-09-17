import { cn } from "@/lib/utils";

interface StripedPatternProps extends React.SVGProps<SVGSVGElement> {
  width?: number;
  height?: number;
  x?: number;
  y?: number;
  strokeWidth?: number;
  angle?: number;
  className?: string;
}

export function StripedPattern({
  width = 8,
  height = 8,
  x = 0,
  y = 0,
  strokeWidth = 1,
  angle = -45,
  className,
  ...props
}: StripedPatternProps) {
  const id = "striped-pattern";

  return (
    <svg
      aria-hidden="true"
      className={cn(
        "pointer-events-none absolute inset-0 h-full w-full fill-line-soft stroke-line-soft",
        className
      )}
      {...props}
    >
      <defs>
        <pattern
          id={id}
          width={width}
          height={height}
          patternUnits="userSpaceOnUse"
          patternTransform={`rotate(${angle})`}
          x={x}
          y={y}
        >
          <line x1="0" y1="0" x2="0" y2={height} strokeWidth={strokeWidth} />
        </pattern>
      </defs>
      <rect width="100%" height="100%" strokeWidth={0} fill={`url(#${id})`} />
    </svg>
  );
}

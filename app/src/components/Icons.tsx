interface IconProps {
  size?: number;
}

export function HomeIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M4 10.4 12 4l8 6.4V19a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1z" />
    </svg>
  );
}

export function MenuIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M4 6.5h16M4 12h16M4 17.5h16" />
    </svg>
  );
}

export function PlusIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M12 5v14M5 12h14" />
    </svg>
  );
}

export function MinusIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      aria-hidden="true"
    >
      <path d="M5 12h14" />
    </svg>
  );
}

function Glyph({ size = 13, d, width = "1.8" }: IconProps & { d: string; width?: string }) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={width}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d={d} />
    </svg>
  );
}

export const ChevronLeftIcon = (p: IconProps = {}) => <Glyph {...p} d="M14.5 6l-6 6 6 6" />;
export const ChevronRightIcon = (p: IconProps = {}) => <Glyph {...p} d="M9.5 6l6 6-6 6" />;
export const CheckIcon = (p: IconProps = {}) => <Glyph {...p} d="M5 12.5l4.5 4.5L19 7.5" width="2.2" />;
export const CloseIcon = (p: IconProps = {}) => <Glyph {...p} d="M6.5 6.5l11 11M17.5 6.5l-11 11" />;
export const StarIcon = (p: IconProps = {}) => (
  <Glyph {...p} d="M12 3.86l2.35 5.76 6.21.46-4.76 4.02 1.49 6.04L12 16.86l-5.29 3.28 1.49-6.04-4.76-4.02 6.21-.46Z" />
);
export const GridIcon = (p: IconProps = {}) => (
  <Glyph {...p} d="M4.5 4.5h6v6h-6zM13.5 4.5h6v6h-6zM4.5 13.5h6v6h-6zM13.5 13.5h6v6h-6z" />
);
export const ListIcon = (p: IconProps = {}) => (
  <Glyph {...p} d="M4 6.5h1.2M8 6.5h12M4 12h1.2M8 12h12M4 17.5h1.2M8 17.5h12" />
);

export function BattleIcon({ size = 12, className }: IconProps & { className?: string }) {
  return (
    <svg className={className} width={size} height={size} fill="currentColor" viewBox="0 0 24 24" aria-hidden="true">
      <path d="M4 9.001h13l-1.6 1.2a1 1 0 1 0 1.2 1.6l4-3a1.001 1.001 0 0 0 0-1.59l-3.86-3a1.001 1.001 0 1 0-1.23 1.58l1.57 1.21H4a1 1 0 1 0 0 2Z" />
      <path d="M20 16H7l1.6-1.2a1 1 0 1 0-1.2-1.6l-4 3a1 1 0 0 0 0 1.59l3.86 3a1 1 0 0 0 1.05.107 1 1 0 0 0 .35-.287 1 1 0 0 0-.17-1.4L6.92 18H20a1 1 0 0 0 0-2Z" />
    </svg>
  );
}

export function PencilIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      className="pencil"
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M14.3632 5.65156L15.8431 4.17157C16.6242 3.39052 17.8905 3.39052 18.6716 4.17157L20.0858 5.58579C20.8668 6.36683 20.8668 7.63316 20.0858 8.41421L18.6058 9.8942M14.3632 5.65156L4.74749 15.2672C4.41542 15.5993 4.21079 16.0376 4.16947 16.5054L3.92738 19.2459C3.87261 19.8659 4.39148 20.3848 5.0115 20.33L7.75191 20.0879C8.21972 20.0466 8.65806 19.8419 8.99013 19.5099L18.6058 9.8942M14.3632 5.65156L18.6058 9.8942" />
    </svg>
  );
}

export function FilterIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20.4 4.75H3.6l6.9 7.9v4.85l3 1.75v-6.6l6.9-7.9Z" />
    </svg>
  );
}

export function ArchiveIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M3.5 7.25h17v2.5h-17z" />
      <path d="M5 9.75v8.5a1.5 1.5 0 0 0 1.5 1.5h11a1.5 1.5 0 0 0 1.5-1.5v-8.5" />
      <path d="M10 13.25h4" />
      <path d="M3.5 7.25 5.4 4.6a1.5 1.5 0 0 1 1.22-.62h10.76a1.5 1.5 0 0 1 1.22.62l1.9 2.65" />
    </svg>
  );
}

export function TrashIcon({ size = 13 }: IconProps = {}) {
  return (
    <svg
      width={size}
      height={size}
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <path d="M20 9L18.005 20.3463C17.8369 21.3026 17.0062 22 16.0353 22H7.96474C6.99379 22 6.1631 21.3026 5.99496 20.3463L4 9" />
      <path d="M21 6L15.375 6M3 6L8.625 6M8.625 6V4C8.625 2.89543 9.52043 2 10.625 2H13.375C14.4796 2 15.375 2.89543 15.375 4V6M8.625 6L15.375 6" />
    </svg>
  );
}

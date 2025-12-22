import LogoSvg from "../assets/logo.svg?react";

interface LogoProps {
	size?: number;
	className?: string;
}

export function Logo({ size = 48, className }: LogoProps) {
	return (
    <LogoSvg
      width={size}
      height={size}
      className={className}
      role="img"
      aria-label="Voice logo"
      /*
				The SVG paths use `currentColor`, so we intentionally avoid hard-coding
				fill. This allows the logo to pick up Tangerine accents via CSS.
			*/
    />
  );
}

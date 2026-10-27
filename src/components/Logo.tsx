export function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg className="logo" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id="logo-gradient" x1="4" y1="2" x2="28" y2="30" gradientUnits="userSpaceOnUse">
          <stop stopColor="#6d7bff" />
          <stop offset="1" stopColor="#8a4dff" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="8" fill="url(#logo-gradient)" />
      <circle cx="9" cy="16" r="3.2" fill="#fff" />
      <circle cx="23" cy="16" r="3.2" fill="#fff" fillOpacity=".92" />
      <path d="M13.2 16h5.6" stroke="#fff" strokeWidth="2" strokeLinecap="round" strokeDasharray="0.1 3.6" />
    </svg>
  );
}

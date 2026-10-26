/** The app icon (`src-tauri/icons/icon.svg`) drawn for small sizes: a tunnel with light at the end. */
export function Logo({ size = 28 }: { size?: number }) {
  return (
    <svg className="logo" width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id="logo-bg" x1="3" y1="0" x2="29" y2="32" gradientUnits="userSpaceOnUse">
          <stop stopColor="#8479ff" />
          <stop offset=".5" stopColor="#5247e8" />
          <stop offset="1" stopColor="#281b98" />
        </linearGradient>
        <linearGradient id="logo-outer" x1="0" y1="6" x2="0" y2="27" gradientUnits="userSpaceOnUse">
          <stop stopColor="#1a1370" />
          <stop offset="1" stopColor="#241a8c" />
        </linearGradient>
        <linearGradient id="logo-middle" x1="0" y1="9" x2="0" y2="25" gradientUnits="userSpaceOnUse">
          <stop stopColor="#3126b8" />
          <stop offset="1" stopColor="#4b40da" />
        </linearGradient>
        <linearGradient id="logo-inner" x1="0" y1="12" x2="0" y2="24" gradientUnits="userSpaceOnUse">
          <stop stopColor="#6573ff" />
          <stop offset="1" stopColor="#95aaff" />
        </linearGradient>
        <linearGradient id="logo-floor" x1="0" y1="23" x2="0" y2="27" gradientUnits="userSpaceOnUse">
          <stop stopColor="#8ff0ff" stopOpacity=".75" />
          <stop offset="1" stopColor="#8ff0ff" stopOpacity="0" />
        </linearGradient>
      </defs>
      <rect width="32" height="32" rx="8" fill="url(#logo-bg)" />
      <path d="M6.7 27.2V15.1a9.3 9.3 0 0 1 18.6 0v12.1Z" fill="url(#logo-outer)" />
      <path d="M9.2 25.5V16.2a6.8 6.8 0 0 1 13.6 0v9.3Z" fill="url(#logo-middle)" />
      <path d="M11.4 24.1v-7.2a4.6 4.6 0 0 1 9.2 0v7.2Z" fill="url(#logo-inner)" />
      <path d="M13.5 22.9h5l6.8 4.3H6.7Z" fill="url(#logo-floor)" />
      <path d="M13.5 22.9v-4.8a2.5 2.5 0 0 1 5 0v4.8Z" fill="#fff" />
    </svg>
  );
}

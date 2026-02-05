export function GeminiIcon({ className }: { className?: string }) {
  return (
    <svg
      className={className}
      viewBox="0 0 24 24"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <path
        d="M12 0C12 6.627 6.627 12 0 12c6.627 0 12 5.373 12 12 0-6.627 5.373-12 12-12-6.627 0-12-5.373-12-12Z"
        fill="url(#gemini-gradient)"
      />
      <defs>
        <linearGradient
          id="gemini-gradient"
          x1="0"
          y1="0"
          x2="24"
          y2="24"
          gradientUnits="userSpaceOnUse"
        >
          <stop stopColor="#4285F4" />
          <stop offset="1" stopColor="#886FBF" />
        </linearGradient>
      </defs>
    </svg>
  )
}

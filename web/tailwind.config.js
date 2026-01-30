/** @type {import('tailwindcss').Config} */
export default {
    darkMode: ["class"],
    content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
  	extend: {
  		colors: {
  			cream: {
  				'50': 'hsl(45, 30%, 98%)',
  				'100': 'hsl(45, 30%, 96%)',
  				'200': 'hsl(45, 25%, 92%)',
  				'300': 'hsl(45, 20%, 88%)',
  				DEFAULT: 'hsl(45, 30%, 96%)'
  			},
  			charcoal: {
  				'50': 'hsl(45, 8%, 35%)',
  				'100': 'hsl(45, 8%, 28%)',
  				'200': 'hsl(45, 10%, 20%)',
  				'300': 'hsl(45, 10%, 16%)',
  				'400': 'hsl(45, 10%, 12%)',
  				DEFAULT: 'hsl(45, 10%, 12%)'
  			},
  			warm: {
  				DEFAULT: 'hsl(35, 25%, 55%)',
  				light: 'hsl(35, 30%, 70%)',
  				muted: 'hsl(35, 15%, 65%)'
  			},
  			terracotta: {
  				DEFAULT: 'hsl(15, 35%, 50%)',
  				light: 'hsl(15, 30%, 65%)',
  				muted: 'hsl(15, 20%, 55%)'
  			},
  			sage: {
  				DEFAULT: 'hsl(90, 15%, 45%)',
  				light: 'hsl(90, 15%, 60%)',
  				muted: 'hsl(90, 10%, 55%)'
  			},
  			stone: {
  				DEFAULT: 'hsl(30, 8%, 50%)',
  				light: 'hsl(30, 10%, 65%)',
  				muted: 'hsl(30, 5%, 60%)'
  			},
  			success: {
  				DEFAULT: 'hsl(90, 20%, 45%)',
  				light: 'hsl(90, 15%, 55%)'
  			},
  			error: {
  				DEFAULT: 'hsl(10, 40%, 50%)',
  				light: 'hsl(10, 35%, 60%)'
  			},
  			card: {
  				DEFAULT: 'hsl(var(--card))',
  				hover: 'hsla(45, 30%, 100%, 0.85)',
  				foreground: 'hsl(var(--card-foreground))'
  			},
  			background: 'hsl(var(--background))',
  			foreground: 'hsl(var(--foreground))',
  			popover: {
  				DEFAULT: 'hsl(var(--popover))',
  				foreground: 'hsl(var(--popover-foreground))'
  			},
  			primary: {
  				DEFAULT: 'hsl(var(--primary))',
  				foreground: 'hsl(var(--primary-foreground))'
  			},
  			secondary: {
  				DEFAULT: 'hsl(var(--secondary))',
  				foreground: 'hsl(var(--secondary-foreground))'
  			},
  			muted: {
  				DEFAULT: 'hsl(var(--muted))',
  				foreground: 'hsl(var(--muted-foreground))'
  			},
  			accent: {
  				DEFAULT: 'hsl(var(--accent))',
  				foreground: 'hsl(var(--accent-foreground))'
  			},
  			destructive: {
  				DEFAULT: 'hsl(var(--destructive))',
  				foreground: 'hsl(var(--destructive-foreground))'
  			},
  			border: 'hsl(var(--border))',
  			input: 'hsl(var(--input))',
  			ring: 'hsl(var(--ring))',
  			chart: {
  				'1': 'hsl(var(--chart-1))',
  				'2': 'hsl(var(--chart-2))',
  				'3': 'hsl(var(--chart-3))',
  				'4': 'hsl(var(--chart-4))',
  				'5': 'hsl(var(--chart-5))'
  			}
  		},
  		fontFamily: {
  			display: [
  				'"Cormorant Garamond"',
  				'Georgia',
  				'serif'
  			],
  			sans: [
  				'Outfit',
  				'system-ui',
  				'sans-serif'
  			]
  		},
  		fontSize: {
  			label: [
  				'10px',
  				{
  					letterSpacing: '0.2em',
  					lineHeight: '1.4'
  				}
  			]
  		},
  		borderRadius: {
  			none: '0',
  			sm: 'calc(var(--radius) - 4px)',
  			DEFAULT: '4px',
  			md: 'calc(var(--radius) - 2px)',
  			lg: 'var(--radius)'
  		},
  		boxShadow: {
  			subtle: '0 1px 3px rgba(40, 38, 34, 0.04)',
  			card: '0 1px 2px rgba(40, 38, 34, 0.03)'
  		},
  		animation: {
  			'fade-up': 'fadeUp 0.6s ease-out forwards',
  			'fade-in': 'fadeIn 0.5s ease-out forwards'
  		},
  		keyframes: {
  			fadeUp: {
  				'0%': {
  					opacity: '0',
  					transform: 'translateY(16px)'
  				},
  				'100%': {
  					opacity: '1',
  					transform: 'translateY(0)'
  				}
  			},
  			fadeIn: {
  				'0%': {
  					opacity: '0'
  				},
  				'100%': {
  					opacity: '1'
  				}
  			}
  		},
  		spacing: {
  			'18': '4.5rem',
  			'22': '5.5rem'
  		}
  	}
  },
  plugins: [require("tailwindcss-animate"), require("@tailwindcss/typography")],
}

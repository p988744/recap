import reflex as rx

# Custom daisyUI theme: Gentle Paw
gentle_paw_theme = {
    "gentle-paw": {
        # Base Colors (背景層次)
        "base-100": "#F5F2E8",      # Bone Ivory
        "base-200": "#EBE8DE",      # 稍深一點的米色
        "base-300": "#DDD9CC",      # 更深的米色
        "base-content": "#3D2832",  # Velvet Collar

        # Primary (主色調) - Golden Fur
        "primary": "#B8864A",
        "primary-content": "#FDF9F3",

        # Secondary (次要色) - Mohogany Bark
        "secondary": "#6B2A2A",
        "secondary-content": "#F5F2E8",

        # Accent (強調色) - Tranquil Sky
        "accent": "#A8B8C8",
        "accent-content": "#2A3542",

        # Neutral (中性色) - Gentle Paw
        "neutral": "#8A8478",
        "neutral-content": "#F5F2E8",

        # Semantic Colors
        "info": "#A8B8C8",          # Tranquil Sky
        "info-content": "#2A3542",
        "success": "#6B7355",       # Leaf Green
        "success-content": "#F5F2E8",
        "warning": "#B8864A",       # Golden Fur
        "warning-content": "#3D2832",
        "error": "#C25A3C",         # Rustic Tail
        "error-content": "#F5F2E8",

        # Border Radius
        "--rounded-box": "0.75rem",
        "--rounded-btn": "0.5rem",
        "--rounded-badge": "1rem",

        # Animation
        "--animation-btn": "0.2s",
        "--animation-input": "0.2s",

        # Focus ring
        "--btn-focus-scale": "0.98",
    },
}

# Tailwind + daisyUI configuration
tailwind_config = {
    "theme": {
        "extend": {
            "colors": {
                "background": "rgb(var(--background))",
                "foreground": "rgb(var(--foreground))",
            },
        },
    },
    "plugins": ["daisyui"],
    "daisyui": {
        "themes": [gentle_paw_theme],
        "darkTheme": "gentle-paw",
        "base": True,
        "styled": True,
        "utils": True,
    },
}

config = rx.Config(
    app_name="app",
    title="Tempo Worklog 分析器",
    plugins=[
        rx.plugins.TailwindV3Plugin(config=tailwind_config),
    ],
    disable_plugins=["reflex.plugins.sitemap.SitemapPlugin"],
)

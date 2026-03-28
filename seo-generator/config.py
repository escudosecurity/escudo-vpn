"""Configuration for the SEO page generator."""

import os
from pathlib import Path

# Paths
BASE_DIR = Path(__file__).parent
PROJECT_ROOT = BASE_DIR.parent
SITE_DIR = PROJECT_ROOT / "site"
OUTPUT_DIR = SITE_DIR / "seguranca-digital"
TEMPLATE_DIR = BASE_DIR / "templates"
CACHE_DIR = BASE_DIR / "cache"

# Database
DB_CONFIG = {
    "host": os.environ.get("SEO_DB_HOST", "144.76.2.72"),
    "port": int(os.environ.get("SEO_DB_PORT", "5432")),
    "dbname": os.environ.get("SEO_DB_NAME", "enlace"),
    "user": os.environ.get("SEO_DB_USER", "enlace"),
    "password": os.environ.get("SEO_DB_PASSWORD", "enlace"),
}

# Site
SITE_URL = "https://escudovpn.com"
SEO_BASE_PATH = "/seguranca-digital"

# Vulnerability Index weights
VULN_WEIGHTS = {
    "speed": 0.25,
    "mobile_tech": 0.20,
    "safety": 0.20,
    "infrastructure": 0.15,
    "provider_concentration": 0.10,
    "tower_density": 0.10,
}

# Mobile tech scores (higher = more vulnerable)
MOBILE_TECH_SCORES = {
    "2G": 100,
    "3G": 60,
    "4G": 20,
    "5G": 0,
}

# Score classifications
SCORE_LABELS = {
    (0, 25): ("Baixo risco", "green"),
    (26, 50): ("Risco moderado", "yellow"),
    (51, 75): ("Alto risco", "orange"),
    (76, 100): ("Risco critico", "red"),
}

# Score CSS colors
SCORE_COLORS = {
    "green": "#2ea043",
    "yellow": "#d29922",
    "orange": "#db6d28",
    "red": "#da3633",
}

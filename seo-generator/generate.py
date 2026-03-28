#!/usr/bin/env python3
"""Escudo VPN — Programmatic SEO page generator.

Generates static HTML pages for all Brazilian municipalities with
Digital Vulnerability Index scores and local internet infrastructure data.

Usage:
    python3 generate.py                    # Full generation from DB
    python3 generate.py --state pr         # Only Parana
    python3 generate.py --sample-data      # Use sample data (no DB needed)
    python3 generate.py --sample-data --state pr  # Sample data, only PR
"""

import argparse
import os
import sys
from collections import defaultdict
from datetime import date
from pathlib import Path

from jinja2 import Environment, FileSystemLoader

from config import OUTPUT_DIR, SITE_DIR, SITE_URL, SEO_BASE_PATH, TEMPLATE_DIR
from models import Municipality
from scores import calculate_vulnerability


def parse_args():
    parser = argparse.ArgumentParser(description="Generate SEO pages for Escudo VPN")
    parser.add_argument("--state", type=str, help="Generate only for this state (e.g., pr, sp)")
    parser.add_argument("--sample-data", action="store_true", help="Use sample data instead of DB")
    parser.add_argument("--clean", action="store_true", help="Remove existing output before generating")
    return parser.parse_args()


def setup_jinja() -> Environment:
    env = Environment(
        loader=FileSystemLoader(str(TEMPLATE_DIR)),
        autoescape=False,
        trim_blocks=True,
        lstrip_blocks=True,
    )
    return env


def compute_all_scores(munis: dict):
    """Calculate vulnerability scores for all municipalities."""
    # Compute national average towers per 1k population
    total_towers = 0
    total_pop = 0
    for m in munis.values():
        total_towers += m.towers.total_towers
        total_pop += m.population
    nat_avg = (total_towers / total_pop * 1000) if total_pop > 0 else 0.5

    for m in munis.values():
        m.vulnerability = calculate_vulnerability(m, nat_avg)


def get_pct_4g5g(m: Municipality) -> float:
    """Get percentage of 4G+5G subscribers."""
    total = m.mobile.total_subscribers
    if total == 0:
        return 0.0
    subs_4g5g = 0
    for tech, subs in m.mobile.tech_subscribers.items():
        t = tech.upper()
        if "4G" in t or "5G" in t or "LTE" in t or "NR" in t:
            subs_4g5g += subs
    return round(subs_4g5g / total * 100, 1)


def get_hero_text(m: Municipality) -> str:
    """Generate dynamic hero paragraph based on top risk factors."""
    factors = []

    if m.vulnerability.speed_score > 60:
        factors.append(f"velocidade de internet abaixo da media ({m.speed.avg_download:.1f} Mbps)")
    if m.vulnerability.mobile_tech_score > 60:
        pct_legacy = 100 - get_pct_4g5g(m)
        factors.append(f"{pct_legacy:.0f}% dos assinantes em redes 2G/3G vulneraveis")
    if m.vulnerability.safety_score > 60:
        factors.append(f"indice de criminalidade elevado (score {m.safety.risk_score:.0f}/100)")
    if m.vulnerability.infrastructure_score > 60:
        factors.append(f"infraestrutura digital limitada ({m.infrastructure.school_internet_pct:.0f}% das escolas com internet)")
    if m.vulnerability.tower_score > 60:
        factors.append("baixa densidade de antenas de celular")
    if m.vulnerability.provider_score > 60:
        factors.append(f"apenas {len(m.mobile.providers)} operadoras moveis")

    if not factors:
        factors.append(f"velocidade media de {m.speed.avg_download:.1f} Mbps")

    top2 = factors[:2]
    risk_text = " e ".join(top2)

    return (
        f"{m.name} apresenta indice de vulnerabilidade digital de {m.vulnerability.total:.0f}/100 "
        f"({m.vulnerability.label}), com {risk_text}. "
        f"Veja a analise completa e como proteger sua conexao."
    )


def get_cta_text(m: Municipality) -> str:
    """Generate CTA text contextualized to the city's risks."""
    if m.vulnerability.total >= 76:
        return (
            f"Com indice de vulnerabilidade critico ({m.vulnerability.total:.0f}/100), "
            f"moradores de {m.name} precisam de protecao extra. O Escudo VPN criptografa "
            f"todo seu trafego com tecnologia pos-quantica, bloqueia malware e protege "
            f"seus dados mesmo em redes 2G/3G."
        )
    elif m.vulnerability.total >= 51:
        return (
            f"{m.name} apresenta risco alto de vulnerabilidade digital. "
            f"O Escudo VPN protege sua conexao com criptografia de nivel militar, "
            f"bloqueio de anuncios e malware, e streaming desbloqueado."
        )
    elif m.vulnerability.total >= 26:
        return (
            f"Mesmo com risco moderado, {m.name} pode se beneficiar de protecao extra. "
            f"O Escudo VPN adiciona uma camada de seguranca com criptografia pos-quantica."
        )
    else:
        return (
            f"{m.name} tem boa infraestrutura digital, mas ameacas online nao dependem "
            f"de localizacao. O Escudo VPN protege contra phishing, rastreadores e malware em qualquer rede."
        )


def render_municipality(env: Environment, m: Municipality, generated_date: str) -> str:
    template = env.get_template("municipality.html")
    return template.render(
        m=m,
        pct_4g5g=get_pct_4g5g(m),
        hero_text=get_hero_text(m),
        cta_text=get_cta_text(m),
        generated_date=generated_date,
    )


def render_state(env: Environment, state, municipalities: list, generated_date: str) -> str:
    template = env.get_template("state.html")
    sorted_munis = sorted(municipalities, key=lambda x: x.name)
    top_vulnerable = sorted(municipalities, key=lambda x: x.vulnerability.total, reverse=True)
    avg_score = sum(m.vulnerability.total for m in municipalities) / len(municipalities) if municipalities else 0
    avg_speed = sum(m.speed.avg_download for m in municipalities) / len(municipalities) if municipalities else 0
    total_pop = sum(m.population for m in municipalities)

    return template.render(
        state=state,
        municipalities=sorted_munis,
        top_vulnerable=top_vulnerable,
        avg_score=avg_score,
        avg_speed=avg_speed,
        total_pop=total_pop,
        generated_date=generated_date,
    )


def render_national(env: Environment, states: dict, munis: dict, generated_date: str) -> str:
    template = env.get_template("national.html")

    all_munis = list(munis.values())
    total_munis = len(all_munis)
    avg_score = sum(m.vulnerability.total for m in all_munis) / total_munis if total_munis else 0
    avg_speed = sum(m.speed.avg_download for m in all_munis) / total_munis if total_munis else 0

    # Group by state
    by_state = defaultdict(list)
    for m in all_munis:
        by_state[m.state_id].append(m)

    state_stats = []
    for sid, state in sorted(states.items(), key=lambda x: x[1].name):
        state_munis = by_state.get(sid, [])
        if not state_munis:
            continue
        state_stats.append({
            "name": state.name,
            "abbrev": state.abbrev,
            "muni_count": len(state_munis),
            "avg_score": sum(m.vulnerability.total for m in state_munis) / len(state_munis),
            "avg_speed": sum(m.speed.avg_download for m in state_munis) / len(state_munis),
        })

    top_vulnerable = sorted(all_munis, key=lambda x: x.vulnerability.total, reverse=True)

    return template.render(
        total_munis=total_munis,
        avg_score=avg_score,
        avg_speed=avg_speed,
        state_stats=state_stats,
        top_vulnerable=top_vulnerable,
        generated_date=generated_date,
    )


def generate_sitemap(munis: dict, states: dict):
    """Generate sitemap-seo.xml and sitemap-index.xml."""
    urls = []

    # National index
    urls.append(f"{SITE_URL}{SEO_BASE_PATH}/")

    # State pages
    state_abbrevs = set()
    for m in munis.values():
        state_abbrevs.add(m.state_abbrev)
    for abbrev in sorted(state_abbrevs):
        urls.append(f"{SITE_URL}{SEO_BASE_PATH}/{abbrev}/")

    # Municipality pages
    for m in sorted(munis.values(), key=lambda x: (x.state_abbrev, x.slug)):
        urls.append(f"{SITE_URL}{SEO_BASE_PATH}/{m.state_abbrev}/{m.slug}")

    today = date.today().isoformat()

    # sitemap-seo.xml
    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">')
    for url in urls:
        lines.append(f"  <url><loc>{url}</loc><lastmod>{today}</lastmod><changefreq>monthly</changefreq></url>")
    lines.append("</urlset>")

    sitemap_seo = SITE_DIR / "sitemap-seo.xml"
    sitemap_seo.write_text("\n".join(lines), encoding="utf-8")
    print(f"  {sitemap_seo} ({len(urls)} URLs)")

    # sitemap-main.xml (existing pages)
    existing_pages = [
        f"{SITE_URL}/",
        f"{SITE_URL}/comparativo.html",
        f"{SITE_URL}/vazamentos.html",
        f"{SITE_URL}/privacy.html",
        f"{SITE_URL}/termos.html",
    ]
    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append('<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">')
    for url in existing_pages:
        lines.append(f"  <url><loc>{url}</loc><lastmod>{today}</lastmod></url>")
    lines.append("</urlset>")

    sitemap_main = SITE_DIR / "sitemap-main.xml"
    sitemap_main.write_text("\n".join(lines), encoding="utf-8")
    print(f"  {sitemap_main} ({len(existing_pages)} URLs)")

    # sitemap-index.xml
    lines = ['<?xml version="1.0" encoding="UTF-8"?>']
    lines.append('<sitemapindex xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">')
    lines.append(f"  <sitemap><loc>{SITE_URL}/sitemap-main.xml</loc><lastmod>{today}</lastmod></sitemap>")
    lines.append(f"  <sitemap><loc>{SITE_URL}/sitemap-seo.xml</loc><lastmod>{today}</lastmod></sitemap>")
    lines.append("</sitemapindex>")

    sitemap_index = SITE_DIR / "sitemap-index.xml"
    sitemap_index.write_text("\n".join(lines), encoding="utf-8")
    print(f"  {sitemap_index}")


def generate_robots_txt():
    """Generate robots.txt."""
    content = """User-agent: Googlebot
Allow: /

User-agent: GPTBot
Allow: /

User-agent: OAI-SearchBot
Allow: /

User-agent: *
Allow: /

Sitemap: https://escudovpn.com/sitemap-index.xml
"""
    robots = SITE_DIR / "robots.txt"
    robots.write_text(content, encoding="utf-8")
    print(f"  {robots}")


def clean_output():
    """Remove existing generated files."""
    import shutil
    if OUTPUT_DIR.exists():
        shutil.rmtree(OUTPUT_DIR)
        print(f"Cleaned {OUTPUT_DIR}")


def main():
    args = parse_args()

    if args.clean:
        clean_output()

    # Load data
    if args.sample_data:
        from sample_data import generate_sample_data
        states, munis = generate_sample_data(args.state)
    else:
        from queries import load_all_data
        states, munis = load_all_data(args.state)

    if not munis:
        print("No municipalities found. Exiting.")
        sys.exit(1)

    # Compute vulnerability scores
    print("Computing vulnerability scores...")
    compute_all_scores(munis)

    # Setup Jinja2
    env = setup_jinja()
    generated_date = date.today().isoformat()

    # Group municipalities by state
    by_state = defaultdict(list)
    for m in munis.values():
        by_state[m.state_id].append(m)

    # Generate municipality pages
    print("Generating municipality pages...")
    muni_count = 0
    for m in munis.values():
        state_dir = OUTPUT_DIR / m.state_abbrev
        state_dir.mkdir(parents=True, exist_ok=True)
        out_path = state_dir / f"{m.slug}.html"
        html = render_municipality(env, m, generated_date)
        out_path.write_text(html, encoding="utf-8")
        muni_count += 1

    print(f"  {muni_count} municipality pages generated")

    # Generate state pages
    print("Generating state pages...")
    state_count = 0
    for state_id, state_munis in by_state.items():
        state = states.get(state_id)
        if not state:
            continue
        state_dir = OUTPUT_DIR / state.abbrev
        state_dir.mkdir(parents=True, exist_ok=True)
        out_path = state_dir / "index.html"
        html = render_state(env, state, state_munis, generated_date)
        out_path.write_text(html, encoding="utf-8")
        state_count += 1

    print(f"  {state_count} state pages generated")

    # Generate national index
    print("Generating national index...")
    OUTPUT_DIR.mkdir(parents=True, exist_ok=True)
    out_path = OUTPUT_DIR / "index.html"
    html = render_national(env, states, munis, generated_date)
    out_path.write_text(html, encoding="utf-8")
    print("  National index generated")

    # Generate sitemaps
    print("Generating sitemaps...")
    generate_sitemap(munis, states)

    # Generate robots.txt
    print("Generating robots.txt...")
    generate_robots_txt()

    # Summary
    total_pages = muni_count + state_count + 1
    print(f"\nDone! {total_pages} pages generated in {OUTPUT_DIR}")
    print(f"  Municipality pages: {muni_count}")
    print(f"  State pages: {state_count}")
    print(f"  National index: 1")


if __name__ == "__main__":
    main()

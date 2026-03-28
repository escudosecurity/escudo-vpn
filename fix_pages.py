#!/usr/bin/env python3
"""
Fix all customer-facing pages: replace nav + footer, fix colors/fonts.
"""

import re
import os

SITE_DIR = '/home/dev/pulsovpn/escudo-vpn/site'

# ── The new NAV (exact from index.html) ──────────────────────────────────────
NEW_NAV = '''<nav>
  <div class="nav-inner">
    <a href="/" class="nav-logo">
      <div class="logo-mark">
        <svg viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
          <path d="M12 3L4 8v7c0 5 3.5 9 8 11 4.5-2 8-6 8-11V8l-8-5z" fill="#c9a84c"/>
        </svg>
      </div>
      <span class="logo-text">Escudo</span>
    </a>
    <ul class="nav-links">
      <li><a href="#recursos">Recursos</a></li>
      <li><a href="#planos">Planos</a></li>
      <li><a href="/ferramentas.html">Ferramentas</a></li>
      <li><a href="/servidores.html">Servidores</a></li>
    </ul>
    <a href="/cadastro.html" class="nav-cta">Comece agora</a>
    <button class="nav-toggle" aria-label="Menu" onclick="document.querySelector('.nav-links').style.display=document.querySelector('.nav-links').style.display==='flex'?'none':'flex'">
      <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <line x1="3" y1="6" x2="21" y2="6"/><line x1="3" y1="12" x2="21" y2="12"/><line x1="3" y1="18" x2="21" y2="18"/>
      </svg>
    </button>
  </div>
</nav>'''

# ── The new FOOTER (exact from index.html) ───────────────────────────────────
NEW_FOOTER = '''<footer style="background:#0a0a0a;position:relative;overflow:hidden">
  <!-- Gold abstract background -->
  <div style="position:absolute;inset:0;background:url('img/hero-gold-v2.png') center/cover no-repeat;opacity:0.06;pointer-events:none"></div>
  <div style="position:absolute;inset:0;background:linear-gradient(180deg,#0a0a0a 0%,transparent 30%,transparent 70%,#0a0a0a 100%);pointer-events:none"></div>

  <!-- CTA band -->
  <div style="position:relative;max-width:1100px;margin:0 auto;padding:56px 24px 40px;text-align:center;border-bottom:1px solid rgba(201,168,76,0.1)">
    <div style="font-size:28px;font-weight:800;letter-spacing:-1px;margin-bottom:8px;color:#fff">Pronto para navegar com liberdade?</div>
    <div style="font-size:14px;color:rgba(255,255,255,0.35);margin-bottom:24px">Teste grátis por 7 dias. Sem cartão de crédito. Pague com PIX.</div>
    <a href="/cadastro.html" style="display:inline-block;padding:14px 36px;border-radius:100px;background:linear-gradient(135deg,#c9a84c,#8b6914);color:#000;font-weight:700;font-size:14px;text-decoration:none">Comece agora</a>
  </div>

  <!-- Footer content -->
  <div class="footer-grid" style="position:relative">

    <div class="footer-brand">
      <div class="footer-logo">
        <div class="logo-mark" style="background:#003322;width:40px;height:40px;border-radius:10px;display:flex;align-items:center;justify-content:center">
          <img src="img/logo-v1.png" alt="Escudo" style="width:40px;height:40px;border-radius:10px">
        </div>
        <span class="logo-text" style="font-size:20px;font-weight:800;color:#fff">Escudo</span>
      </div>
      <p class="footer-tagline" style="color:rgba(255,255,255,0.4);line-height:1.7;margin:12px 0 16px;max-width:300px">A primeira VPN brasileira com IPs residenciais reais. Privacidade, streaming e proteção — sem complicação.</p>
      <div style="display:inline-flex;align-items:center;gap:6px;padding:6px 14px;border-radius:20px;background:rgba(0,51,34,0.3);border:1px solid rgba(201,168,76,0.15);font-size:10px;font-weight:600;color:rgba(201,168,76,0.7)">
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="#c9a84c" stroke-width="2"><path d="M12 3L4 8v7c0 5 3.5 9 8 11 4.5-2 8-6 8-11V8l-8-5z"/></svg>
        Feito no Brasil &middot; Registrado no Panamá
      </div>
    </div>

    <div class="footer-col">
      <h4 style="color:rgba(201,168,76,0.6)">Produto</h4>
      <ul>
        <li><a href="#recursos">Recursos</a></li>
        <li><a href="#planos">Planos</a></li>
        <li><a href="/download.html">Download</a></li>
        <li><a href="/servidores.html">Servidores</a></li>
        <li><a href="/comparativo.html">Comparativo</a></li>
      </ul>
    </div>

    <div class="footer-col">
      <h4 style="color:rgba(201,168,76,0.6)">Ferramentas</h4>
      <ul>
        <li><a href="/teste-de-velocidade.html">Teste de velocidade</a></li>
        <li><a href="/meu-ip.html">Meu IP</a></li>
        <li><a href="/verificar-senha.html">Verificar senha</a></li>
        <li><a href="/verificar-link.html">Verificar link</a></li>
        <li><a href="/scanner.html">Scanner de ameaças</a></li>
        <li><a href="/teste-de-privacidade.html">Teste de privacidade</a></li>
      </ul>
    </div>

    <div class="footer-col">
      <h4 style="color:rgba(201,168,76,0.6)">Empresa</h4>
      <ul>
        <li><a href="/sobre.html">Sobre nós</a></li>
        <li><a href="/blog/">Blog</a></li>
        <li><a href="/ajuda.html">Central de ajuda</a></li>
        <li><a href="/o-que-e-vpn.html">O que é VPN?</a></li>
        <li><a href="/privacy.html">Privacidade</a></li>
        <li><a href="/termos.html">Termos de uso</a></li>
      </ul>
    </div>

  </div>
  <div class="footer-bottom" style="position:relative;border-top:1px solid rgba(201,168,76,0.08)">
    <span style="color:rgba(255,255,255,0.2)">Escudo VPN 2026. Todos os direitos reservados.</span>
    <span>
      <a href="/privacy.html" style="color:rgba(201,168,76,0.3)">Privacidade</a>
      &middot;
      <a href="/termos.html" style="color:rgba(201,168,76,0.3)">Termos</a>
    </span>
  </div>
</footer>'''

# ── NAV CSS to inject if page uses inline <style> ────────────────────────────
NAV_CSS = '''
    /* ── NAV ── */
    nav {
      position: sticky;
      top: 0;
      z-index: 100;
      background: rgba(255,255,255,0.96);
      backdrop-filter: blur(16px);
      -webkit-backdrop-filter: blur(16px);
      border-bottom: 1px solid rgba(0,0,0,0.06);
    }
    .nav-inner {
      max-width: 1100px;
      margin: 0 auto;
      padding: 0 24px;
      height: 64px;
      display: flex;
      align-items: center;
      gap: 32px;
    }
    .nav-logo {
      display: flex;
      align-items: center;
      gap: 10px;
      text-decoration: none;
      flex-shrink: 0;
    }
    .logo-mark {
      width: 32px;
      height: 32px;
      background: #003322;
      border-radius: 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
    }
    .logo-mark svg { width: 18px; height: 18px; }
    .logo-text {
      font-weight: 800;
      font-size: 1.1rem;
      color: #111;
      letter-spacing: -0.3px;
    }
    .nav-links {
      display: flex;
      align-items: center;
      gap: 4px;
      list-style: none;
      margin-left: auto;
    }
    .nav-links a {
      font-size: 0.875rem;
      font-weight: 500;
      color: rgba(17,17,17,0.55);
      text-decoration: none;
      padding: 7px 13px;
      border-radius: 8px;
      transition: color .15s, background .15s;
    }
    .nav-links a:hover { color: #111; background: rgba(0,0,0,0.04); }
    .nav-cta {
      margin-left: 8px;
      background: linear-gradient(135deg, #c9a84c, #8b6914) !important;
      color: #fff !important;
      border-radius: 100px !important;
      padding: 9px 22px !important;
      font-weight: 700 !important;
      font-size: 0.875rem !important;
      text-decoration: none;
      transition: opacity .15s;
      flex-shrink: 0;
    }
    .nav-cta:hover { opacity: 0.88; }
    .nav-toggle {
      display: none;
      background: none;
      border: none;
      cursor: pointer;
      padding: 4px;
      color: #111;
      margin-left: auto;
    }
    @media (max-width: 768px) {
      .nav-links { display: none; }
      .nav-toggle { display: block; }
    }
    /* ── FOOTER ── */
    .footer-grid {
      max-width: 1100px;
      margin: 0 auto;
      display: grid;
      grid-template-columns: 2.2fr 1fr 1fr 1fr;
      gap: 40px;
      padding: 40px 24px 32px;
    }
    .footer-brand .footer-logo {
      display: flex;
      align-items: center;
      gap: 10px;
      margin-bottom: 14px;
    }
    .footer-col h4 {
      font-size: 10px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.12em;
      color: rgba(255,255,255,0.5);
      margin-bottom: 14px;
    }
    .footer-col ul { list-style: none; }
    .footer-col ul li { margin-bottom: 9px; }
    .footer-col ul li a {
      font-size: 12px;
      color: rgba(255,255,255,0.34);
      text-decoration: none;
      transition: color .15s;
    }
    .footer-col ul li a:hover { color: #c9a84c; }
    .footer-bottom {
      max-width: 1100px;
      margin: 28px auto 0;
      padding: 20px 24px;
      border-top: 1px solid rgba(255,255,255,0.06);
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 10px;
      color: rgba(255,255,255,0.2);
    }
    .footer-bottom a {
      color: rgba(255,255,255,0.2);
      text-decoration: none;
      transition: color .15s;
    }
    .footer-bottom a:hover { color: rgba(255,255,255,0.5); }
    @media (max-width: 768px) {
      .footer-grid { grid-template-columns: 1fr 1fr; gap: 28px; }
    }
    @media (max-width: 480px) {
      .footer-grid { grid-template-columns: 1fr; }
      .footer-bottom { flex-direction: column; gap: 8px; text-align: center; }
    }
'''

# ── FONT link ────────────────────────────────────────────────────────────────
FONT_LINK = '  <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700;800&family=JetBrains+Mono:wght@400;500;700&display=swap" rel="stylesheet">'

PAGES = [
    'teste-de-velocidade.html',
    'meu-ip.html',
    'scanner.html',
    'verificar-senha.html',
    'verificar-link.html',
    'vazamentos.html',
    'teste-de-privacidade.html',
    'download.html',
    'servidores.html',
    'comparativo.html',
    'sobre.html',
    'ajuda.html',
    'bloqueador-de-anuncios.html',
    'o-que-e-vpn.html',
]

def replace_nav(html):
    """Replace everything from <nav> to </nav> (first occurrence)."""
    # Pattern: from <nav or  <nav (with leading whitespace) to </nav>
    pattern = re.compile(r'\s*<nav\b[^>]*>.*?</nav>', re.DOTALL)
    new_html, count = pattern.subn('\n' + NEW_NAV, html, count=1)
    if count == 0:
        print("  WARNING: no <nav> found!")
    return new_html

def replace_footer(html):
    """Replace everything from <footer> to </footer>."""
    pattern = re.compile(r'\s*<footer\b[^>]*>.*?</footer>', re.DOTALL)
    new_html, count = pattern.subn('\n' + NEW_FOOTER, html, count=1)
    if count == 0:
        print("  WARNING: no <footer> found!")
    return new_html

def fix_colors(html):
    """Fix old green backgrounds in inline styles."""
    # Replace old dark green backgrounds (not in logo-mark which is intentional)
    # We keep #003322 only in logo-mark context
    # These are inline style backgrounds that should be white/off-white
    html = re.sub(r'(background(?:-color)?:\s*)#002a1c\b', r'\1#ffffff', html)
    # Fix body background
    html = re.sub(r'(body\s*\{[^}]*background:\s*)#002a1c', r'\1#ffffff', html)
    html = re.sub(r'(body\s*\{[^}]*background:\s*)#003322', r'\1#ffffff', html)
    return html

def fix_emojis(html):
    """Remove or replace common emojis in visible text."""
    emoji_map = {
        '🔒': '',
        '🛡️': '',
        '🛡': '',
        '⚡': '',
        '✅': '',
        '❌': '',
        '🔍': '',
        '🌍': '',
        '🌐': '',
        '📱': '',
        '💻': '',
        '🚀': '',
        '⭐': '',
        '🔑': '',
        '🎯': '',
        '📊': '',
        '🔥': '',
        '💡': '',
        '🏆': '',
        '👁': '',
        '👁️': '',
        '📍': '',
        '🌎': '',
        '🔓': '',
        '⚠️': '',
        '⚠': '',
        '✓': '',  # keep this, it's not emoji
        '☑': '',
        '🇧🇷': '',
        '🎉': '',
        '💬': '',
        '📧': '',
        '📞': '',
        '🕐': '',
        '🕑': '',
        '👍': '',
        '👎': '',
    }
    for emoji, replacement in emoji_map.items():
        html = html.replace(emoji, replacement)
    return html

def ensure_fonts(html):
    """Ensure Inter + JetBrains Mono fonts are linked."""
    if 'JetBrains+Mono' not in html and 'JetBrains Mono' not in html:
        # Add after existing font links or after charset
        html = re.sub(
            r'(<link[^>]*fonts\.googleapis[^>]*>\s*)',
            r'\1' + FONT_LINK + '\n',
            html, count=1
        )
        if FONT_LINK not in html:
            html = html.replace(
                '</head>',
                FONT_LINK + '\n</head>',
                1
            )
    # Update font weights to include 800 if missing
    html = re.sub(
        r'(family=Inter:wght@)[\d;]+',
        r'\g<1>400;500;600;700;800',
        html
    )
    return html

def inject_nav_footer_css(html):
    """If page has inline <style>, inject nav/footer CSS so it works without style.css."""
    # Check if page references style.css - if yes, those classes are already defined
    has_style_css = 'style.css' in html

    if not has_style_css:
        # Inject nav/footer CSS into existing <style> block
        html = re.sub(
            r'(<style[^>]*>)',
            r'\1' + NAV_CSS,
            html, count=1
        )
    else:
        # Pages with style.css - but the nav CSS uses .nav-inner vs .wrap
        # Add override CSS for the new nav structure
        override_css = '''
    /* ── Nav inner override ── */
    .nav-inner {
      max-width: 1100px;
      margin: 0 auto;
      padding: 0 24px;
      height: 64px;
      display: flex;
      align-items: center;
      gap: 32px;
    }
    .nav-cta {
      margin-left: 8px;
      background: linear-gradient(135deg, #c9a84c, #8b6914) !important;
      color: #fff !important;
      border-radius: 100px !important;
      padding: 9px 22px !important;
      font-weight: 700 !important;
      font-size: 0.875rem !important;
      text-decoration: none;
      transition: opacity .15s;
      flex-shrink: 0;
    }
    /* ── Footer override ── */
    .footer-grid {
      max-width: 1100px;
      margin: 0 auto;
      display: grid;
      grid-template-columns: 2.2fr 1fr 1fr 1fr;
      gap: 40px;
      padding: 40px 24px 32px;
    }
    .footer-col h4 {
      font-size: 10px;
      font-weight: 700;
      text-transform: uppercase;
      letter-spacing: 0.12em;
      color: rgba(201,168,76,0.6);
      margin-bottom: 14px;
    }
    .footer-col ul li a { color: rgba(255,255,255,0.34); }
    .footer-col ul li a:hover { color: #c9a84c; }
    .footer-bottom {
      max-width: 1100px;
      margin: 28px auto 0;
      padding: 20px 24px;
      display: flex;
      justify-content: space-between;
      align-items: center;
      font-size: 10px;
      color: rgba(255,255,255,0.2);
    }
    @media (max-width: 768px) {
      .footer-grid { grid-template-columns: 1fr 1fr; gap: 28px; }
      .nav-links { display: none; }
      .nav-toggle { display: block; }
    }
    @media (max-width: 480px) {
      .footer-grid { grid-template-columns: 1fr; }
      .footer-bottom { flex-direction: column; gap: 8px; text-align: center; }
    }
'''
        if '<style>' in html or '<style ' in html:
            html = re.sub(
                r'(<style[^>]*>)',
                r'\1' + override_css,
                html, count=1
            )
        else:
            # No inline style, insert one before </head>
            html = html.replace(
                '</head>',
                f'<style>{override_css}</style>\n</head>',
                1
            )
    return html

modified_count = 0

for page in PAGES:
    path = os.path.join(SITE_DIR, page)
    if not os.path.exists(path):
        print(f"SKIP (not found): {page}")
        continue

    with open(path, 'r', encoding='utf-8') as f:
        original = f.read()

    html = original
    html = replace_nav(html)
    html = replace_footer(html)
    html = fix_colors(html)
    html = fix_emojis(html)
    html = ensure_fonts(html)
    html = inject_nav_footer_css(html)

    if html != original:
        with open(path, 'w', encoding='utf-8') as f:
            f.write(html)
        print(f"UPDATED: {page}")
        modified_count += 1
    else:
        print(f"UNCHANGED: {page}")

print(f"\nDone. {modified_count} files modified.")

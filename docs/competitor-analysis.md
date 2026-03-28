# Escudo VPN -- Competitor Analysis Report
### NordVPN / ProtonVPN / Surfshark
> Generated 2026-03-23 | Based on 143 screenshots across 3 competitor apps

---

## 1. Screen-by-Screen Catalog

### 1.1 Onboarding Screens

| Screen | Nord | Proton | Surfshark |
|--------|------|--------|-----------|
| Use-case survey ("What brings you to...") | Yes -- 5 options (privacy, public Wi-Fi safety, content, restricted networks, gaming) | No | No |
| Threat Protection promo | Yes -- shield + lightning graphic, purple gradient | No | No |
| Dark Web Monitor promo | Yes -- red eye/target graphic | No | No |
| Choose preferred location (smart / manual) | Yes -- "Fastest server" powered by nexos.ai, or specific location | No | No |
| Specific location picker (A-Z country list) | Yes -- flag icons, city names, "Virtual" labels | No | No |
| Ready-to-go first-connect screen | Yes -- map view, "Secure my connection" CTA | No | No |
| Notification permission request | No | No | Yes -- "Allow notifications to stay secure" bottom sheet |
| Battery optimization dialog | No | No | Yes -- "Stop optimising battery usage?" system dialog |
| VPN setup wizard | No | No | Yes -- "Set up VPN" card with illustration |

**Key takeaway:** NordVPN has the most elaborate onboarding (4-step wizard with step indicators). Surfshark focuses on system permissions. ProtonVPN skips onboarding entirely (goes straight to home).

---

### 1.2 Home / Connection Screen

| Element | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| Map view | Yes -- light, outline-style map with numbered clusters and green dot for connected location | Yes -- dark theme, pink/purple gradient map with glowing location dot | No map -- uses a circular radar/pulse animation |
| Connection status | "Secured" (green) / "Not secured" (red) | "You are unprotected" (open padlock, pink) / "Protected" | "Connected" / "Not connected -- Unprotected. Connect to stay safe." |
| IP address display | Yes -- shown at top of map card (e.g. IP 193.176.127.123) | Yes -- shown below country (e.g. Brazil - 187.114.52.189) | Yes -- shown as "VPN IP address" in stats area |
| Server info | Server number (e.g. Brazil #109), connection time (6s) | Auto-selected from flags + country count | Country + city (e.g. Brazil, Sao Paulo) |
| Connect/Disconnect button | "Secure my connection" (blue pill) / "Pause" + "..." menu | "Connect" (purple pill) | "Connect" / "Disconnect" + "Pause" buttons side by side |
| Pause VPN feature | Yes -- via Pause button | No (not visible) | Yes -- bottom sheet with 5/15/30/60 min and 24h options, plus Disconnect |
| Recent locations carousel | Yes -- horizontal scroll cards with flags (Brazil, US, UK) | No | Yes -- "Recently used locations" as horizontal pill chips |
| Dedicated IP upsell | Yes -- banner card below map: "Get your dedicated IP" | No | No |
| Statistics section | "Statistics" label visible at bottom | No | Yes -- connection time, VPN IP, uploaded KB, downloaded KB, protocol in use, LAN discovery |
| Search bar | Yes -- "Search all locations" at top | No (search icon in Countries tab) | No on home (search in location picker) |
| Bottom navigation | 4 tabs: Globe (map), Shield (threat protection), Grid (products), Person (profile) | 4 tabs: Home, Countries, Profiles, Settings | 4 tabs: Home, Products, News, Settings |
| Map card border | Green glow/border when connected | N/A (full-screen dark map) | N/A (no map) |
| Fullscreen toggle | Yes -- expand icon on map card | No | No |

**Key takeaway:** Nord has the richest home screen (map + search + recents + stats + upsell). Proton is map-centric but minimal. Surfshark has no map -- uses a unique radar animation with detailed connection stats below.

---

### 1.3 Server List / Location Picker

| Element | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| Access method | Full-screen page via "All locations" | Dedicated "Countries" tab | Bottom sheet modal "Select a location" |
| Tab filters | Country, Dedicated IP, Meshnet, Obfuscated (horizontal scroll) | All, Secure Core, P2P, Tor | Locations, Static IP, MultiHop |
| Sort/filter | Yes -- filter icon | By category tabs | No visible sort |
| Country list style | Flag + country name + city count, expandable chevron | Flag + country name + "..." menu for each | Flag + country + city, star icon for favorites |
| City-level selection | Yes -- expandable per country (e.g. US: 52 cities) | Yes (implied by country drill-down) | Yes -- Fastest location + Nearest country at top |
| Virtual server labels | Yes -- "Virtual" tag on some countries | No (not visible) | Yes -- "Virtual location" label |
| Search | Search bar at top | Search icon (top right) | Search bar at top of modal |
| Favorites | No visible star/favorite | No visible star/favorite | Yes -- star icon per server to add favorites |
| Server load indicator | No | No | No |
| Secure Core (double hop) | No | Yes -- 68 Secure Core countries, dedicated tab with info card | No |
| P2P filter | No dedicated filter | Yes -- 127 P2P countries, with BitTorrent info card | No |
| Tor integration | No | Yes -- Tor tab for .onion routing | No |
| MultiHop | No (Meshnet for device linking) | No | Yes -- custom double-VPN routes (e.g. "Paris via UK - London") with create/delete |
| Obfuscated servers | Yes -- dedicated tab | No | No |
| Static IP | No | No | Yes -- dedicated tab (e.g. Germany Frankfurt servers listed) |

**Key takeaway:** Each app has a unique specialty. Nord has Obfuscated + Meshnet. Proton has Secure Core + P2P + Tor. Surfshark has MultiHop + Static IP + Favorites.

---

### 1.4 Security & Protection Features

| Feature | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| Threat Protection (ad/malware blocking) | Yes -- dedicated tab with shield UI, "Active" badge, powered by nexos.ai | Yes (called "NetShield") -- toggle in settings | No dedicated screen (included in plans) |
| Always-on option | Yes -- "Only with VPN" or "Always" radio buttons | No (implied by toggle) | No |
| Adult site blocking | Yes -- toggle under Threat Protection | No | No |
| Dark Web Monitor | Yes -- in Products tab, shows "No leaks" status | No | No (Surfshark One has "Alert" for data leaks) |
| Kill Switch | Yes -- in Settings, text description | Yes -- in Settings | Yes -- toggle on connected screen, "Cuts off internet when VPN drops" |
| Split Tunneling | Yes -- "Exclude specific apps from VPN" | Yes -- toggle in Settings (VPN Plus required) | No (not visible) |
| DNS settings | Yes -- "DNS: Default" in settings | No (not visible in screenshots) | No |
| Local Network Discovery | Yes -- toggle in settings | No | Yes -- "Discover on LAN: On" on connected screen |
| Post-Quantum Encryption | Yes -- dedicated setting + info modal, requires NordLynx protocol | No | No (but "quantum-safe" label on WireGuard protocol) |
| VPN Accelerator | No | Yes -- toggle in Settings (VPN Plus) | No |
| Tapjacking Protection | Yes -- toggle in settings | No | No |
| Unsafe Wi-Fi Detection | Yes -- "Get alerts on unsafe Wi-Fi" toggle | No | No |

**Key takeaway:** Nord leads in security feature count. Proton has unique Secure Core (double encryption). Surfshark labels its WireGuard as "quantum-safe" -- a marketing differentiator.

---

### 1.5 Settings Screens

| Setting | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| Preferred location | Yes | No | No |
| Protocol selection | Yes -- "Automatic (recommended)" | Yes -- "Smart (auto)" | WireGuard (shown on connected screen) |
| Auto-connect | Yes -- "On all networks" | No (Connection preferences NEW badge) | No |
| Appearance/Theme | Yes -- "System" | Yes -- "Dark" with Theme option | No |
| Push notifications | Yes -- toggle + preferences link | Yes -- in General section | No |
| App icon customization | No | Yes -- "App icon" option | No |
| Widget | No | Yes -- "Widget NEW" | No |
| Privacy preferences | Yes -- in settings footer | No | No |
| Terms | Yes | No | Yes (in subscription info) |
| App version | Yes -- "9.0.4" | No | No |
| Debug/Diagnostic logs | No | Yes -- "Debug logs" | No |
| Help fight censorship | No | Yes -- "Help us fight censorship" option | No |
| Rate app | No | Yes -- "Rate Proton VPN" | No |
| Advanced settings | No | Yes -- separate "Advanced settings" section | No |

---

### 1.6 Products / Extra Features Hub

| Feature | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| Products/Features hub | Yes -- dedicated "Products" tab | No (features in Settings) | Yes -- "Products" tab in bottom nav |
| Dedicated IP upsell | Yes -- prominent card with "Get dedicated IP" blue CTA | No | No |
| Threat Protection status | Yes -- "On" badge with manage link | Via NetShield toggle | No |
| Dark Web Monitor status | Yes -- "On" badge, "No leaks" | No | No |
| Meshnet | Yes -- "Off, Turn on to link devices" | No | No |
| Password Manager upsell | Yes -- "Get NordPass" | No | No |
| Cloud Storage upsell | Yes -- "Get NordLocker" | No | No |
| AI tools upsell | Yes -- "Try premium AI models, Get nexos.ai" | No | No |
| eSIM/travel data | Yes -- "Try Saily for easy travel data" | No | No |
| Alternative ID | No | No | Yes -- "Create alternative identity" with persona + email generator |
| Antivirus | No | No | Yes (Surfshark One tier) |
| Private search | No | No | Yes -- "Browse tracking-free with Search" (One tier) |
| Email leak monitoring | No | No | Yes (One tier) |
| Credit card leak monitoring | No | No | Yes (One tier) |
| ID leak monitoring | No | No | Yes (One tier) |
| Masked email generator | No | No | Yes (Starter + One) |
| Personal detail generator | No | No | Yes (Starter + One) |

**Key takeaway:** Nord has the deepest ecosystem (NordPass, NordLocker, nexos.ai, Saily). Surfshark bundles security tools into tiered plans (Starter vs One). Proton bundles separately (Proton Unlimited = VPN + Mail + Calendar + Drive + Pass).

---

### 1.7 Account / Profile Screens

| Element | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| Profile page | Yes -- dedicated "Profile" tab | No (Create account / Sign in in Settings) | No dedicated profile page |
| Manage account | Yes -- email shown, "Manage account" link | Via external "Create an account" or "Sign in" | No |
| Settings access | Yes -- from Profile tab | Yes -- dedicated bottom tab | Yes -- dedicated bottom tab |
| Help center | Yes -- "Help" with quick fixes (Reconnect, Change protocol, Exclude app) | Yes -- "Support center" external link | No |
| Report issue | Yes -- "Report a website issue" | Yes -- "Report an issue" | No |
| Suggest a feature | Yes | No | No |
| Diagnostic logs | Yes | Yes -- "Debug logs" | No |
| Refer a friend | Yes -- full page with referral link, 3 months reward, step-by-step explanation | No | No |
| Notifications center | Yes -- dedicated page ("You're all caught up!") | Yes -- in Settings | No |

---

### 1.8 Subscription / Pricing Screens

| Element | Nord | Proton | Surfshark |
|---------|------|--------|-----------|
| In-app pricing | Via Dedicated IP upsell web view | Yes -- dark theme paywall with feature carousel (11 slides) | Yes -- teal-themed paywall, plan comparison |
| Tier structure | Single tier + Dedicated IP add-on | VPN Plus / Proton Unlimited | Surfshark Starter / Surfshark One |
| Free tier | No | Yes -- free with limited servers and speed | No (7-day free trial) |
| Plan toggle | N/A | N/A | Yes -- 12 months / 6 months / 1 month tabs |
| Feature comparison table | Yes (Shared IP vs Dedicated IP web page) | No (carousel slides) | Yes -- detailed Starter vs One comparison table |
| Trial | No | No | Yes -- 7-day free trial for Starter |
| Promotional pricing | N/A shown | -65% (1 year), -87% (1 month welcome) | Varies by period |

---

### 1.9 Connection Profiles (Proton Unique)

| Profile | Description |
|---------|-------------|
| Streaming US | United States, optimized for streaming |
| Gaming | Fastest country |
| Anti-censorship | Fastest country excluding user's country |
| Max security | Via Secure Core |
| Work/School | Fastest country |
| Custom | "Create profile" CTA |

This is a **unique Proton feature** -- preset connection profiles for different use cases.

---

### 1.10 Upsell / Marketing Screens

| Screen | Nord | Proton | Surfshark |
|--------|------|--------|-----------|
| Dedicated IP web view | Yes -- full marketing page with benefits (skip blocklists, no CAPTCHAs, remote access), comparison table, location list (30+ countries) | No | No |
| VPN Plus paywall carousel | No | Yes -- 11 slides: Speed, Stream 4K, Ad-free (NetShield), Secure Core, P2P, 10 devices, Tor, Split tunneling, Profiles, Advanced customization (LAN, Custom DNS, NAT) | No |
| Proton Unlimited paywall | No | Yes -- all Proton products (VPN + Mail + Calendar + Drive + Pass) | No |
| Surfshark Starter paywall | No | No | Yes -- plan cards with trial CTA |
| Surfshark One comparison | No | No | Yes -- feature-by-feature Starter vs One matrix |

---

## 2. Feature Matrix

| Feature | NordVPN | ProtonVPN | Surfshark | Escudo Has | Escudo Needs |
|---------|---------|-----------|-----------|------------|--------------|
| **Core VPN** | | | | | |
| One-tap connect | Yes | Yes | Yes | Yes | -- |
| Map-based interface | Yes (light, outline) | Yes (dark, gradient) | No (radar animation) | No | YES -- Priority |
| Server search | Yes | Yes | Yes | Basic | Improve |
| Country list with flags | Yes | Yes | Yes | Yes | -- |
| City-level selection | Yes (52 US cities) | Yes | Yes | No | YES |
| Favorite servers | No | No | Yes (star) | No | YES |
| Recent servers | Yes (carousel) | No | Yes (chips) | No | YES |
| Pause VPN | Yes | No | Yes (5/15/30/60m/24h) | No | YES |
| Connection timer | No | No | Yes | No | YES |
| Upload/Download stats | No | No | Yes | No | YES |
| IP address display | Yes | Yes | Yes | No | YES |
| **Protocols & Encryption** | | | | | |
| Auto protocol selection | Yes | Yes (Smart) | Yes (WireGuard default) | No | YES |
| WireGuard | Yes (NordLynx) | Yes | Yes | No | YES |
| Post-quantum encryption | Yes (toggle + explainer) | No | Yes (labeled "quantum-safe") | No | Consider |
| **Security Features** | | | | | |
| Kill Switch | Yes | Yes | Yes | No | YES -- Priority |
| Split Tunneling | Yes | Yes (Plus) | No visible | No | YES |
| Threat Protection / Ad blocker | Yes (dedicated) | Yes (NetShield) | No (in One tier) | No | YES |
| Dark Web Monitor | Yes | No | Yes (One tier Alert) | No | Consider |
| Obfuscated servers | Yes | No | No | No | Consider |
| Secure Core (double encryption) | No | Yes (68 countries) | No | No | Consider |
| MultiHop (custom double VPN) | No | No | Yes (create custom routes) | No | Consider |
| DNS configuration | Yes | No | No | No | Consider |
| LAN Discovery toggle | Yes | No | Yes | No | YES |
| Unsafe Wi-Fi detection | Yes | No | No | No | Consider |
| Tapjacking protection | Yes | No | No | No | Optional |
| Auto-connect | Yes | No | No | No | YES |
| **Identity & Privacy** | | | | | |
| Alternative ID / persona | No | No | Yes (generate fake identity) | No | Consider |
| Masked email generator | No | No | Yes | No | Consider |
| Antivirus | No | No | Yes (One) | No | No |
| Private search | No | No | Yes (One) | No | No |
| **Platform & UX** | | | | | |
| Connection Profiles | No | Yes (Streaming, Gaming, Anti-censorship, Max Security, Work) | No | No | YES |
| Widget | No | Yes (NEW) | No | No | Consider |
| Custom app icon | No | Yes | No | No | Optional |
| Theme (light/dark) | Yes (System) | Yes (Dark default) | No (fixed light in VPN, teal accent) | No | Light only (brand) |
| Bottom navigation | 4 tabs | 4 tabs | 4 tabs | 3 tabs | YES -- 4 tabs |
| Refer a friend | Yes (3 months reward) | No | No | No | YES |
| In-app help & quick fixes | Yes (Reconnect, Change protocol, Exclude app) | Yes (Support center, Debug logs) | No | No | YES |
| **Server Types** | | | | | |
| Virtual servers | Yes (labeled) | No visible | Yes (labeled) | No | YES |
| Dedicated IP | Yes (premium add-on, 30+ countries) | No | No | No | Consider |
| Static IP | No | No | Yes | No | Consider |
| P2P-optimized | No filter | Yes (127 countries, dedicated tab) | No | No | Consider |
| Tor-over-VPN | No | Yes | No | No | No |
| Meshnet (device linking) | Yes | No | No | No | No |
| **Ecosystem** | | | | | |
| Password manager | Yes (NordPass) | Yes (Proton Pass) | No | No | No |
| Cloud storage | Yes (NordLocker) | Yes (Proton Drive) | No | No | No |
| Email service | No | Yes (Proton Mail) | No | No | No |
| eSIM / travel data | Yes (Saily) | No | No | No | No |
| AI tools | Yes (nexos.ai) | No | No | No | No |
| **Subscription** | | | | | |
| Free tier | No | Yes | No (7-day trial) | No | Consider trial |
| Multiple plan tiers | No (single + add-on) | Yes (Free / Plus / Unlimited) | Yes (Starter / One) | Single | Consider tiers |
| In-app plan comparison | Yes (web) | Yes (carousel) | Yes (table) | No | YES |

---

## 3. Design Analysis

### 3.1 Color Schemes

| App | Primary | Secondary | Accent | Background | Status Bar |
|-----|---------|-----------|--------|------------|------------|
| **NordVPN** | White (#ffffff) | Light gray (#f5f5f5) | Blue (#4747ff) | White | Light |
| **ProtonVPN** | Dark navy (#1a1a2e) | Dark charcoal (#252540) | Purple (#7b5bff) + Teal/Green (#00d4aa) | Dark | Dark |
| **Surfshark** | White (#ffffff) | Teal/Cyan (#1cd4b0) header gradient | Black (#000000) buttons | White + Teal gradient header | Light (white) or dark (teal) |

**Analysis:**
- Nord uses a clean, light, almost clinical white design -- closest to Escudo's Pearl & Precision direction
- Proton is fully dark-theme by default, premium/moody aesthetic
- Surfshark mixes white body with bold teal headers, uses black CTAs -- more playful/youthful

### 3.2 Typography

| App | Headings | Body | Weight Pattern |
|-----|----------|------|----------------|
| **NordVPN** | Sans-serif, bold, black | Regular weight gray | Heavy heading, light body contrast |
| **ProtonVPN** | Sans-serif, bold, white-on-dark | Light gray body text | Similar contrast approach on dark bg |
| **Surfshark** | Sans-serif, extra bold, very large "Connected"/"Not connected" | Regular weight | Largest headings of all three |

### 3.3 Navigation Patterns

All three use a **4-tab bottom navigation bar**:

| Tab Position | NordVPN | ProtonVPN | Surfshark |
|-------------|---------|-----------|-----------|
| Tab 1 | Globe (Map/VPN) | Home (house) | Home (shield) |
| Tab 2 | Shield (Threat Protection) | Countries (globe) | Products (grid) |
| Tab 3 | Grid (Products) | Profiles (cards) | News (bell) |
| Tab 4 | Person (Profile) | Settings (gear) | Settings (gear) |

**Active indicator styles:**
- Nord: Filled/dark icon, slight background circle
- Proton: Purple filled circle behind icon, label below
- Surfshark: Black filled icon, label below

### 3.4 Map Implementation

| Aspect | NordVPN | ProtonVPN | Surfshark |
|--------|---------|-----------|-----------|
| Map present | Yes | Yes | No |
| Style | Light, outline-only map with blue/green dots | Dark theme with gradient overlay, pink glow at location | Radar/pulse animation instead |
| Interactivity | Numbered clusters (3, 2), tap to expand | Static location indicator | N/A |
| Map card vs full screen | Card with expand button | Full-width in home section | N/A |
| Connected indicator | Green dot on map | Pink/red glowing dot | Teal gradient background fills screen |
| Disconnected look | Light blue/white map, blue dots | Dark map, pink padlock icon above | Gray radar animation |

### 3.5 Connection Button Design

| App | Disconnected State | Connected State |
|-----|-------------------|-----------------|
| **NordVPN** | "Secure my connection" -- blue pill button, full width | "Pause" button (gray pill) + "..." overflow menu |
| **ProtonVPN** | "Connect" -- purple pill button inside a card | N/A (connecting observed via home) |
| **Surfshark** | "Connect" -- black pill button, full width | "Disconnect" (outlined) + "Pause" (teal filled) side by side |

### 3.6 Card Styles and Layouts

| Pattern | NordVPN | ProtonVPN | Surfshark |
|---------|---------|-----------|-----------|
| Card radius | Large (~16px) rounded | Medium rounded | Large rounded |
| Card shadow | Subtle, elevated | Flat (dark-on-dark) | Subtle shadow |
| Map card | Bordered green glow when connected | N/A | N/A |
| Feature cards | White cards on light gray bg, icon left + text right | Dark cards on darker bg | White cards with clean borders |
| Upsell cards | Blue CTA button, icon + title + description | Purple full-width CTA | Black CTA, teal accents |
| Server list items | Flag circle + country + city/count + chevron | Flag + country + "..." | Flag + country + city + star |

### 3.7 Server List Handling

| Pattern | NordVPN | ProtonVPN | Surfshark |
|---------|---------|-----------|-----------|
| Presentation | Full-screen page | Full-screen tab | Bottom sheet modal |
| Grouping | Tabs (Country, Dedicated IP, Meshnet, Obfuscated) | Tabs (All, Secure Core, P2P, Tor) | Tabs (Locations, Static IP, MultiHop) |
| Search | Top search bar | Top-right icon | Top search bar in modal |
| Expand/collapse | Chevron for countries with multiple cities | "..." overflow per country | Direct list |
| Smart options | N/A (smart selection in onboarding) | N/A | "Fastest location" + "Nearest country" at top |

---

## 4. Gap Analysis -- What Escudo is Missing

Features present in 2+ competitors that Escudo lacks, ranked by importance:

### CRITICAL (All 3 competitors have)
1. **Kill Switch** -- All three have it. Non-negotiable for a VPN app.
2. **Protocol selection** (WireGuard / auto) -- All three offer this. Industry standard.
3. **IP address display on home screen** -- All three show current IP. Essential trust signal.
4. **4-tab bottom navigation** -- All three use this pattern. Escudo should match.
5. **Country list with flags and city-level selection** -- All three. Core UX expectation.
6. **Search in server list** -- All three. Required for usability.

### HIGH PRIORITY (2 of 3 competitors have)
7. **Map-based home screen** -- Nord and Proton both use maps. Strong visual differentiator.
8. **Pause VPN** (with timed options) -- Nord and Surfshark. Useful for quick troubleshooting.
9. **Recent/favorite servers** -- Nord (recents) and Surfshark (favorites + recents). Speeds up reconnection.
10. **Split Tunneling** -- Nord and Proton. Power-user feature, expected.
11. **Threat Protection / Ad blocking** -- Nord (dedicated) and Proton (NetShield). Key value-add.
12. **Auto-connect on unsafe networks** -- Nord has it fully. Proton has connection preferences.
13. **LAN Discovery toggle** -- Nord and Surfshark. Needed for local network access while VPN is on.
14. **Connection statistics** (time, data transfer) -- Nord (partial) and Surfshark (detailed). Builds trust.

### MEDIUM PRIORITY (1 competitor has, strong differentiator)
15. **Connection Profiles** (Proton) -- Streaming, Gaming, Work presets. Excellent UX.
16. **MultiHop / Double VPN** -- Surfshark (MultiHop) and Proton (Secure Core). Privacy differentiator.
17. **Post-quantum encryption** -- Nord (toggle) and Surfshark (label). Marketing advantage.
18. **Dark Web / Data Leak monitoring** -- Nord and Surfshark (One). Premium feature.
19. **Dedicated/Static IP** -- Nord (Dedicated IP) and Surfshark (Static IP). Premium add-on.
20. **Refer a friend program** -- Only Nord, but strong growth lever.
21. **Alternative ID / persona generator** -- Surfshark unique. Privacy power feature.
22. **Widget for quick connect** -- Proton unique. Convenience feature.
23. **In-app help with quick fixes** -- Nord. Reduces support tickets.
24. **Virtual server labels** -- Nord and Surfshark. Transparency feature.

### NICE TO HAVE
25. Obfuscated servers (Nord only)
26. Tor-over-VPN (Proton only)
27. Meshnet / device linking (Nord only)
28. Custom app icon (Proton only)
29. eSIM integration (Nord/Saily only)
30. Tapjacking protection (Nord only)

---

## 5. Escudo App Redesign Recommendations

### 5.1 Design Direction -- Pearl & Precision

The Escudo app should feel like a premium financial instrument -- the "Bloomberg Terminal of VPN" on mobile. Apply the Pearl & Precision brand:

**Color system:**
- Base: White (#ffffff)
- Alternate sections: Off-white (#fafaf7)
- CTA buttons: Gold gradient (linear-gradient(135deg, #c9a84c, #8b6914))
- Data/stats blocks: Dark (#111111) for high contrast
- Status: Green (#22c55e) for connected, Red (#ef4444) for disconnected
- Text: #111 headings, #666 body, #999 tertiary

**Typography:**
- Inter 700-800 for headings
- Inter 400-500 for body
- Large status text on home ("Protected" / "Unprotected") at 28-32px bold

**Components:**
- Pill-shaped buttons (border-radius: 100px)
- Cards with subtle shadows (0 2px 8px rgba(0,0,0,0.06))
- Gold gradient accent lines and borders
- NO dark theme -- light base only

**Differentiation from competitors:**
- Nord = utilitarian white/blue
- Proton = dark/techy purple
- Surfshark = playful teal
- **Escudo = luxurious white/gold** -- premium positioning unique in the market

---

### 5.2 Navigation Structure (4 tabs)

```
[Shield]     [Globe]      [Grid]       [Person]
 Escudo      Locations    Products     Account
```

1. **Escudo (Home)** -- Map view, connect/disconnect, status, stats
2. **Locations** -- Server browser with search, filters, favorites
3. **Products** -- Security suite (ad blocker, leak monitor, profiles)
4. **Account** -- Settings, subscription, help, refer a friend

---

### 5.3 Key Screens to Design (Priority Order)

#### Screen 1: Home -- Disconnected
- White background
- Elegant outline map with gold accent dots for server clusters
- Country flag + name + "Unprotected" in red
- Real IP address shown (dimmed)
- Large gold gradient "Connect" pill button
- Recent locations as horizontal pill chips below
- Bottom nav bar

#### Screen 2: Home -- Connected
- Map zooms to connected region
- Green glow on location dot
- Gold-bordered map card showing: IP, Server #, Connection time
- Country flag + city + "Protected" in green
- "Pause" (gold outline) + "..." (overflow) buttons
- Stats bar: Upload/Download speed, connection duration
- Dark (#111) stats block at bottom

#### Screen 3: Server List
- Full-screen overlay (not bottom sheet -- more premium feel)
- Search bar at top
- Filter pills: All / Favorites / Fast / MultiHop
- Country cards: Flag circle, country name, city count, latency indicator
- Gold star for favorites (filled = favorited)
- "Fastest server" smart option at top with gold lightning bolt

#### Screen 4: Quick Connect Profiles
- Inspired by Proton's Profiles but with Escudo branding
- Pre-built profiles: "Daily Browsing", "Streaming", "Work", "Maximum Privacy"
- Custom profile creation
- Each profile: icon + name + description + one-tap connect

#### Screen 5: Security Dashboard (Products tab)
- Clean card layout
- Ad Blocker card: toggle + blocked count
- Leak Monitor card: status + last scan date
- Kill Switch card: toggle + description
- Connection Health: protocol, encryption, DNS status

#### Screen 6: Settings
- Organized sections: Connection, Security, General, Support
- Connection: Protocol, Auto-connect, Preferred location, Split tunneling
- Security: Kill Switch, LAN Discovery, DNS
- General: Notifications, Language
- Support: Help center, Report issue, Diagnostic logs

#### Screen 7: Onboarding (3 screens)
- Welcome + value prop (map illustration with gold pins)
- Choose use case (similar to Nord's survey but simpler)
- Enable permissions + auto-connect setup

#### Screen 8: Subscription/Pricing
- Plan comparison card
- Gold gradient CTA for premium
- Feature checklist with gold checkmarks

---

### 5.4 Map Implementation Approach

**Recommended: Light vector map (SVG) -- inspired by Nord but elevated**

- Use a light, minimal SVG world map with thin gray country borders
- Server locations shown as small gold dots (clustered with count badges)
- Current location: pulsing gold ring animation
- Connected server: green dot with gold ring
- Pan/zoom enabled with smooth animation
- Tap on cluster to expand to individual servers
- Map sits in a rounded card with subtle shadow
- Expand button (top right) for full-screen map view

**Implementation:**
- React Native: use react-native-maps with custom map style, or a custom SVG map component
- Custom SVG approach gives more control over the luxury aesthetic
- Pre-render map tiles in white/gold color scheme

**Why map over radar animation (Surfshark's approach):**
- Maps communicate global reach and server diversity
- 2 of 3 competitors use maps -- it is the industry expectation
- Maps enable geographic server selection (tap a region)
- Aligns with the "Bloomberg" precision aesthetic

---

### 5.5 Feature Implementation Priority

**Phase 1 -- MVP (Must-have to compete)**
1. Map-based home screen with connect/disconnect
2. Kill Switch
3. Protocol selection (WireGuard + auto)
4. IP address display
5. Country/city server list with search
6. Recent and favorite servers
7. 4-tab navigation
8. Basic settings (protocol, auto-connect)

**Phase 2 -- Differentiation**
9. Pause VPN with timed options
10. Connection profiles (Daily, Streaming, Work, Privacy)
11. Threat Protection / Ad blocker
12. Split Tunneling
13. Connection statistics dashboard
14. Onboarding flow

**Phase 3 -- Premium Features**
15. Data Leak Monitor
16. MultiHop (double VPN)
17. Post-quantum encryption label/toggle
18. Refer a friend program
19. Widget for quick connect
20. In-app help with quick fixes

**Phase 4 -- Ecosystem (Future)**
21. Alternative ID / masked email
22. Dedicated IP add-on
23. Multiple subscription tiers

---

### 5.6 Competitive Positioning Summary

| Aspect | Nord | Proton | Surfshark | Escudo (Target) |
|--------|------|--------|-----------|-----------------|
| Theme | Clinical white | Dark & techy | Teal & playful | Luxurious white & gold |
| Audience | Mainstream | Privacy purists | Budget-conscious | Premium Brazilian market |
| Strength | Ecosystem depth | Open-source trust | Price + unlimited devices | Design + local focus |
| Map | Outline, functional | Dark, atmospheric | None (radar) | Elegant, gold-accented |
| Unique hook | nexos.ai + Saily | Secure Core + Tor | Alt ID + MultiHop | Pearl & Precision brand |
| Navigation | 4 tabs | 4 tabs | 4 tabs | 4 tabs |
| CTA color | Blue | Purple | Black | Gold gradient |

---

### 5.7 Critical Design Principles for Escudo

1. **White space is luxury.** Do not cram screens. Let elements breathe like Nord does.
2. **Gold = premium action.** Every CTA, toggle, and accent should use the gold gradient to reinforce brand.
3. **The map IS the product.** Make it the hero of the home screen. It communicates security visually.
4. **Status must be instant.** The user should know their protection status within 0.5 seconds of opening the app -- large "Protected" / "Unprotected" text.
5. **Connection stats build trust.** Show IP, time, speed, protocol. Users who see data feel in control.
6. **Profiles simplify choices.** Instead of making users pick countries, let them pick intents (streaming, privacy, work).
7. **Fewer screens, deeper content.** Nord has too many upsell surfaces. Keep Escudo focused on VPN excellence.

---

*End of competitive analysis. This document drives the Escudo VPN app redesign -- every screen, feature, and design decision should reference this analysis.*

---

## 6. Additional Findings (Manual Review)

### ProtonVPN — Tor Integration
- Dedicated Tor tab in server list
- Routes traffic through VPN → Tor network
- Access .onion sites without Tor browser
- Escudo should consider this for privacy-focused users

### Surfshark — Dynamic Connection Animation
- When connecting, shows a 0% → 100% animated progress indicator
- Smooth, dynamic feel — not just a spinner
- Creates a sense of "something is happening" and builds confidence
- **Escudo MUST have this** — one of the best UX touches across all 3 apps

### Surfshark — MultiHop Custom Routes (Entry + Exit)
- Users pick ENTRY server (country A) and EXIT server (country B)
- Traffic enters through one country, exits through another
- Example: "Paris via UK - London" 
- Users can CREATE and DELETE custom routes
- More flexible than ProtonVPN's Secure Core (which has fixed entry points)
- **Escudo's Double VPN should work this way** — let users pick both ends

### Features to Add to Escudo (Updated Priority)

**MUST HAVE for launch:**
1. Kill Switch
2. IP display on home screen
3. 4-tab bottom navigation
4. City-level server selection with search
5. Dynamic 0-100% connection animation (like Surfshark)
6. Protocol selection (WireGuard auto/manual)
7. Map-based home screen (white + gold, Pearl & Precision)
8. Recent + favorite servers
9. Connection stats (time, data up/down, protocol)

**HIGH PRIORITY (v1.1):**
10. Pause VPN with timer (5/15/30/60 min)
11. Split Tunneling (exclude apps)
12. Escudo Shield in-app (ad/malware blocking toggle)
13. Auto-connect on unsafe WiFi
14. LAN Discovery toggle
15. MultiHop with custom entry+exit (like Surfshark)

**v2.0:**
16. Connection Profiles (Streaming, Gaming, Privacy, Work)
17. Dark Web / Data Leak monitoring
18. Alternative ID / persona generator
19. Refer a friend program
20. Widget for quick connect

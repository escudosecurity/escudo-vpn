# Escudo VPN App — Complete Redesign Specification

> Master document: backend fixes, new features, database changes, API endpoints, Android screens, and design direction.
> Generated 2026-03-23. Based on competitor analysis of NordVPN, ProtonVPN, Surfshark (143 screenshots).

---

## Part 1: Backend Fixes and Additions

### 1.1 Database — New Tables

```sql
-- User favorites (server bookmarks)
CREATE TABLE favorites (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    server_id UUID NOT NULL REFERENCES servers(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(user_id, server_id)
);

-- Connection profiles (presets for Streaming, Gaming, Privacy, Work, Custom)
CREATE TABLE connection_profiles (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    icon VARCHAR(50) NOT NULL DEFAULT 'custom',
    server_id UUID REFERENCES servers(id),
    protocol VARCHAR(30) NOT NULL DEFAULT 'wireguard',
    split_tunnel_apps TEXT[] DEFAULT '{}',
    is_preset BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Referral system
CREATE TABLE referrals (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    referrer_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    referred_id UUID REFERENCES users(id),
    code VARCHAR(20) NOT NULL UNIQUE,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    reward_months INTEGER NOT NULL DEFAULT 3,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    redeemed_at TIMESTAMPTZ
);

-- Per-user settings (synced between devices)
CREATE TABLE user_settings (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    kill_switch BOOLEAN NOT NULL DEFAULT true,
    auto_connect BOOLEAN NOT NULL DEFAULT false,
    auto_connect_wifi_only BOOLEAN NOT NULL DEFAULT true,
    split_tunnel_apps TEXT[] DEFAULT '{}',
    protocol VARCHAR(30) NOT NULL DEFAULT 'wireguard_auto',
    lan_discovery BOOLEAN NOT NULL DEFAULT false,
    preferred_server_id UUID REFERENCES servers(id),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 1.2 Database — Alter Existing Tables

```sql
-- Add latitude/longitude to servers for map display
ALTER TABLE servers ADD COLUMN latitude DOUBLE PRECISION;
ALTER TABLE servers ADD COLUMN longitude DOUBLE PRECISION;
ALTER TABLE servers ADD COLUMN city VARCHAR(100);
ALTER TABLE servers ADD COLUMN country_name VARCHAR(100);
ALTER TABLE servers ADD COLUMN is_virtual BOOLEAN NOT NULL DEFAULT false;

-- Populate coordinates for existing servers
UPDATE servers SET latitude=-23.55, longitude=-46.63, city='Sao Paulo', country_name='Brasil' WHERE country_code='BR';
UPDATE servers SET latitude=40.71, longitude=-74.01, city='New York', country_name='Estados Unidos' WHERE name='nj-01';
UPDATE servers SET latitude=38.90, longitude=-77.04, city='Ashburn', country_name='Estados Unidos' WHERE name='escudo-ashburn';
UPDATE servers SET latitude=45.52, longitude=-122.68, city='Hillsboro', country_name='Estados Unidos' WHERE name='escudo-hillsboro';
UPDATE servers SET latitude=52.37, longitude=4.90, city='Amsterdam', country_name='Paises Baixos' WHERE name='ams-01';
UPDATE servers SET latitude=50.45, longitude=11.10, city='Falkenstein', country_name='Alemanha' WHERE name='escudo-falkenstein';
UPDATE servers SET latitude=49.45, longitude=11.08, city='Nuremberg', country_name='Alemanha' WHERE name='escudo-nuremberg';
UPDATE servers SET latitude=60.17, longitude=24.94, city='Helsinki', country_name='Finlandia' WHERE name='escudo-helsinki';
UPDATE servers SET latitude=51.51, longitude=-0.13, city='Londres', country_name='Reino Unido' WHERE country_code='GB';
UPDATE servers SET latitude=1.35, longitude=103.82, city='Singapura', country_name='Singapura' WHERE country_code='SG';
UPDATE servers SET latitude=-33.87, longitude=151.21, city='Sydney', country_name='Australia' WHERE country_code='AU';
UPDATE servers SET latitude=43.65, longitude=-79.38, city='Toronto', country_name='Canada' WHERE country_code='CA';
UPDATE servers SET latitude=12.97, longitude=77.59, city='Bangalore', country_name='India' WHERE country_code='IN';
UPDATE servers SET latitude=40.42, longitude=-3.70, city='Madrid', country_name='Espanha' WHERE country_code='ES';
UPDATE servers SET latitude=45.46, longitude=9.19, city='Milao', country_name='Italia' WHERE country_code='IT';
UPDATE servers SET latitude=59.33, longitude=18.07, city='Estocolmo', country_name='Suecia' WHERE country_code='SE';
UPDATE servers SET latitude=35.68, longitude=139.69, city='Toquio', country_name='Japao' WHERE country_code='JP';
UPDATE servers SET latitude=-34.60, longitude=-58.38, city='Buenos Aires', country_name='Argentina' WHERE country_code='AR';
UPDATE servers SET latitude=4.71, longitude=-74.07, city='Bogota', country_name='Colombia' WHERE country_code='CO';
UPDATE servers SET latitude=25.20, longitude=55.27, city='Dubai', country_name='Emirados Arabes' WHERE country_code='AE';
UPDATE servers SET latitude=13.76, longitude=100.50, city='Bangkok', country_name='Tailandia' WHERE country_code='TH';
UPDATE servers SET latitude=-6.21, longitude=106.85, city='Jacarta', country_name='Indonesia' WHERE country_code='ID';
UPDATE servers SET latitude=10.82, longitude=106.63, city='Ho Chi Minh', country_name='Vietna' WHERE country_code='VN';
UPDATE servers SET latitude=37.98, longitude=23.73, city='Atenas', country_name='Grecia' WHERE country_code='GR';
UPDATE servers SET latitude=-12.05, longitude=-77.04, city='Lima', country_name='Peru' WHERE country_code='PE';

-- Add connection tracking for recents
ALTER TABLE usage_logs ADD COLUMN IF NOT EXISTS server_name VARCHAR(100);
ALTER TABLE usage_logs ADD COLUMN IF NOT EXISTS server_country VARCHAR(5);
```

### 1.3 API — New Endpoints

**Favorites:**

```
GET    /api/v1/favorites              — List user's favorite servers
POST   /api/v1/favorites              — Add favorite { server_id }
DELETE /api/v1/favorites/:server_id   — Remove favorite
```

**Recents:**

```
GET    /api/v1/recents                — List last 10 connected servers (from usage_logs)
```

**Connection Profiles:**

```
GET    /api/v1/profiles               — List all profiles (presets + custom)
POST   /api/v1/profiles               — Create profile { name, icon, server_id, protocol, split_tunnel_apps }
PUT    /api/v1/profiles/:id           — Update profile
DELETE /api/v1/profiles/:id           — Delete profile (presets cannot be deleted)
```

**User Settings:**

```
GET    /api/v1/settings               — Get user settings
PUT    /api/v1/settings               — Update settings { kill_switch, auto_connect, protocol, split_tunnel_apps, lan_discovery, preferred_server_id }
```

**Referrals:**

```
POST   /api/v1/referral/generate      — Generate referral code (returns code + share link)
GET    /api/v1/referral/status         — Get referral stats (sent, redeemed, months earned)
POST   /api/v1/referral/redeem         — Redeem a referral code { code }
```

**Extended Server List:**

```
GET    /api/v1/servers                 — MODIFY existing: add latitude, longitude, city, country_name, is_virtual to response
```

**Extended DNS Stats:**

```
GET    /api/v1/stats/dns?range=7d     — MODIFY existing: support range parameter (1d, 7d, 30d)
GET    /api/v1/stats/dns/blocked?limit=50  — NEW: list recently blocked domains with timestamps and categories
```

### 1.4 API — Fixes to Existing Endpoints

| Endpoint | Fix Needed |
|----------|-----------|
| `GET /api/v1/servers` | Add `latitude`, `longitude`, `city`, `country_name`, `is_virtual` to JSON response. Group cities under countries. |
| `POST /api/v1/connect` | Log connection to `usage_logs` with `server_name` and `server_country` for recents tracking. |
| `GET /api/v1/stats/dns` | Accept `?range=` query param. Return daily breakdown for charts. |
| `GET /api/v1/account` | Add `subscription_tier`, `subscription_expires`, `referral_code` to response. |
| `POST /api/v1/connect` | Return `server_ip` in response so app can display "Your new IP" on connect. |

---

## Part 2: Android App — New Features

### 2.1 Client-Side Features (No Backend Needed)

| Feature | Implementation |
|---------|---------------|
| **Kill Switch** | Android `VpnService.Builder.setBlocking(true)`. Add always-on VPN intent in settings. Toggle stored in `user_settings`. |
| **Pause VPN** | Local timer (5/15/30/60 min, 24h). Disconnect WireGuard, start countdown, auto-reconnect on expiry. Show countdown in notification. |
| **Connection Timer** | Start `SystemClock.elapsedRealtime()` on connect. Display in HomeScreen. |
| **Upload/Download Stats** | Read from WireGuard interface: `wg show wg0 transfer`. Parse rx/tx bytes. Update every 2 seconds. |
| **Server Ping** | `InetAddress.getByName(ip).isReachable(timeout)` or ICMP ping to each server. Cache results. Show in server list as gold mono text. |
| **Protocol Selection** | Generate WireGuard config with selected options. Options: Auto (default WireGuard), WireGuard, WireGuard + Post-Quantum (Rosenpass config), OpenVPN UDP, OpenVPN TCP. |
| **Split Tunneling** | Android `VpnService.Builder.addDisallowedApplication(packageName)`. Query `PackageManager` for installed apps list. Store excluded apps in `user_settings`. |
| **Auto-Connect** | `WifiMonitor.kt` already exists. Add toggle. When enabled, connect VPN on any WiFi that is not in trusted list. |
| **LAN Discovery** | Android `VpnService.Builder.addRoute()` — exclude local subnet (192.168.x.x, 10.x.x.x) when toggle is on. |
| **0-100% Connection Animation** | Compose animation during `connect()` call. Stages: 0-20% "Conectando ao servidor...", 20-50% "Estabelecendo tunel...", 50-80% "Negociando chaves...", 80-100% "Verificando conexao...", 100% "Conectado". Use `AnimatedVisibility` + `CircularProgressIndicator` with gold gradient. |

### 2.2 Android Screens — New and Redesigned

| Screen | Status | Details |
|--------|--------|---------|
| **HomeScreen.kt** | REDESIGN | Add map (SVG or Mapbox), IP display, stats row, connection animation, recent servers, quick connect cards |
| **ServersScreen.kt** | REDESIGN | Add search, tab filters (Todos/Favoritos/Recentes/Double VPN), city-level expansion, favorites star, ping display |
| **SettingsScreen.kt** | REDESIGN | Grouped sections, gold toggles, protocol picker bottom sheet, split tunneling screen |
| **LoginScreen.kt** | REDESIGN | Pearl and Precision theme |
| **ShieldScreen.kt** | NEW | Dashboard with blocked count, category breakdown, 7-day chart, blocked domains list |
| **ProfilesScreen.kt** | NEW | Preset + custom connection profiles, CRUD |
| **ProfileDetailScreen.kt** | NEW | Profile config editor (server, protocol, split tunnel) |
| **AccountScreen.kt** | NEW | User profile, subscription status, referral banner |
| **SubscriptionScreen.kt** | NEW | Plan comparison (Free vs Pro), upgrade CTA with PIX |
| **OnboardingScreen.kt** | NEW | Welcome + permissions (VPN, notifications) |
| **MultiHopPickerScreen.kt** | NEW | Entry server + Exit server selection for Double VPN |
| **ProtocolPickerSheet.kt** | NEW | Bottom sheet with protocol options and "Mais seguro" badge on post-quantum |
| **SplitTunnelScreen.kt** | NEW | App list with toggles to exclude from VPN |
| **PauseSheet.kt** | NEW | Bottom sheet with timer options (5/15/30/60 min, 24h) |

### 2.3 Android Navigation Update

```kotlin
object Routes {
    const val ONBOARDING = "onboarding"
    const val LOGIN = "login"
    const val HOME = "home"
    const val SERVERS = "servers"
    const val MULTIHOP = "multihop"
    const val SHIELD = "shield"
    const val SHIELD_DETAILS = "shield/details"
    const val PROFILES = "profiles"
    const val PROFILE_DETAIL = "profiles/{id}"
    const val SETTINGS = "settings"
    const val PROTOCOL_PICKER = "settings/protocol"
    const val SPLIT_TUNNEL = "settings/split-tunnel"
    const val ACCOUNT = "account"
    const val SUBSCRIPTION = "subscription"
}

// Bottom nav tabs: Home, Servers, Shield, Settings
```

### 2.4 Android API Service — New Methods

```kotlin
interface ApiService {
    // Existing (keep all current endpoints)

    // Favorites
    @GET("api/v1/favorites")
    suspend fun getFavorites(): List<Server>

    @POST("api/v1/favorites")
    suspend fun addFavorite(@Body body: FavoriteRequest): Response<Unit>

    @DELETE("api/v1/favorites/{serverId}")
    suspend fun removeFavorite(@Path("serverId") serverId: String): Response<Unit>

    // Recents
    @GET("api/v1/recents")
    suspend fun getRecents(): List<Server>

    // Profiles
    @GET("api/v1/profiles")
    suspend fun getProfiles(): List<ConnectionProfile>

    @POST("api/v1/profiles")
    suspend fun createProfile(@Body body: CreateProfileRequest): ConnectionProfile

    @PUT("api/v1/profiles/{id}")
    suspend fun updateProfile(@Path("id") id: String, @Body body: UpdateProfileRequest): ConnectionProfile

    @DELETE("api/v1/profiles/{id}")
    suspend fun deleteProfile(@Path("id") id: String): Response<Unit>

    // Settings
    @GET("api/v1/settings")
    suspend fun getSettings(): UserSettings

    @PUT("api/v1/settings")
    suspend fun updateSettings(@Body body: UserSettings): UserSettings

    // Referrals
    @POST("api/v1/referral/generate")
    suspend fun generateReferral(): ReferralInfo

    @GET("api/v1/referral/status")
    suspend fun getReferralStatus(): ReferralStatus

    @POST("api/v1/referral/redeem")
    suspend fun redeemReferral(@Body body: RedeemRequest): Response<Unit>

    // Extended stats
    @GET("api/v1/stats/dns/blocked")
    suspend fun getBlockedDomains(@Query("limit") limit: Int = 50): List<BlockedDomain>
}
```

---

## Part 3: Design Direction — Pearl and Precision

### 3.1 Color System

| Token | Value | Usage |
|-------|-------|-------|
| `background` | #ffffff | Primary background |
| `surface` | #fafaf7 | Cards, off-white sections |
| `surfaceBorder` | #eeeeee | Card borders |
| `textPrimary` | #111111 | Headings, important text |
| `textSecondary` | #666666 | Body text |
| `textTertiary` | #999999 | Labels, hints |
| `goldStart` | #c9a84c | Gradient start (CTAs, active states) |
| `goldEnd` | #8b6914 | Gradient end |
| `connected` | #22c55e | Connected state, secure indicators |
| `disconnected` | #ef4444 | Disconnected state, danger |
| `warning` | #f59e0b | Caution states |
| `dataBg` | #111111 | Dark data blocks (stats sections) |
| `dataText` | #c9a84c | Gold numbers on dark blocks |
| `navActive` | #c9a84c | Active bottom nav tab |
| `navInactive` | #cccccc | Inactive bottom nav tab |
| `toggleOn` | #c9a84c | Toggle switch active state |
| `toggleOff` | #dddddd | Toggle switch inactive state |
| `chipBg` | #fafaf7 | Pill chips background |
| `chipBorder` | #eeeeee | Pill chips border |

### 3.2 Typography

| Element | Font | Weight | Size |
|---------|------|--------|------|
| App title (navbar) | Inter | 700 | 18sp |
| Screen title | Inter | 800 | 24sp |
| Section header | Inter | 700 | 16sp |
| Card title | Inter | 600 | 15sp |
| Body text | Inter | 400 | 14sp |
| Secondary text | Inter | 400 | 13sp |
| Label / hint | Inter | 500 | 11sp |
| Stat number (large) | JetBrains Mono | 700 | 40sp |
| Stat number (medium) | JetBrains Mono | 700 | 20sp |
| Stat label | Inter | 500 | 10sp |
| Button text | Inter | 700 | 15sp |
| Tab label | Inter | 600 | 10sp |
| Chip text | Inter | 500 | 12sp |
| Ping value | JetBrains Mono | 500 | 12sp |

### 3.3 Component Specifications

**Primary Button (Gold CTA):**
- Background: linear-gradient(135deg, #c9a84c, #8b6914)
- Text: #ffffff, Inter 700, 15sp
- Border radius: 100dp (pill shape)
- Height: 56dp
- Shadow: 0 4dp 16dp rgba(201, 168, 76, 0.3)
- Pressed: opacity 0.9

**Ghost Button:**
- Background: transparent
- Border: 1dp solid #eeeeee
- Text: #666666, Inter 600, 15sp
- Border radius: 100dp
- Height: 56dp

**Card:**
- Background: #fafaf7
- Border: 1dp solid #eeeeee
- Border radius: 16dp
- Padding: 16dp
- Elevation: 0

**Dark Data Block:**
- Background: #111111
- Border radius: 16dp
- Padding: 20dp
- Stat numbers: #c9a84c, JetBrains Mono 700

**Toggle Switch:**
- Track on: #c9a84c
- Track off: #dddddd
- Thumb: #ffffff
- Size: 52dp x 32dp

**Bottom Navigation:**
- Background: #ffffff
- Border top: 1dp solid #eeeeee
- Active icon + text: #c9a84c
- Inactive icon + text: #cccccc
- Height: 64dp
- 4 tabs: Inicio, Servidores, Shield, Config

**Server List Item:**
- Flag emoji + country name (Inter 600 #111) + city count (Inter 400 #999)
- Chevron right for expandable
- Star icon for favorites (filled #c9a84c / outline #ddd)
- Ping: JetBrains Mono 500 #c9a84c

**Recent Server Chip:**
- Background: #fafaf7
- Border: 1dp solid #eeeeee
- Border radius: 100dp
- Padding: 8dp 14dp
- Flag + name (Inter 500 #333) + ping (JetBrains Mono #999)

**Map:**
- Background: #fafaf7
- Continent outlines: #e0e0e0 stroke, no fill
- Server dots: #c9a84c (active has glow shadow)
- Inactive dots: #dddddd
- Active label: white background, shadow, #111 text
- Border radius: 16dp
- Border: 1dp solid #eeeeee

**Connection Animation (0-100%):**
- Circular progress or linear bar
- Gold gradient fill (#c9a84c to #8b6914)
- Large percentage: JetBrains Mono 800 40sp #c9a84c
- Stage text below: Inter 400 13sp #999
- Stages: Conectando (0-20), Estabelecendo tunel (20-50), Negociando chaves (50-80), Verificando (80-100), Conectado (100)

### 3.4 Screen-by-Screen Design Reference

All 19 screens are mockuped at:
`https://escudovpn.com/app-design-full.html`

Screens covered:
1. Home — Disconnected (red warning, gold CTA, quick connect cards)
2. Home — Connecting (0-100% animation, progress stages)
3. Home — Connected (green status, IP, stats, recents)
4. Map — World view (continent outlines, gold server dots)
5. Map — Europe zoom (city labels, ping values)
6. Servers — All countries (search, tabs, flag list)
7. Servers — Country expanded (cities with ping, favorites)
8. Servers — MultiHop picker (entry + exit, saved routes)
9. Shield — Dashboard (blocked count, categories, toggles)
10. Shield — Details (7-day chart, blocked domains list)
11. Settings — Main (grouped sections, gold toggles)
12. Settings — Protocol picker (bottom sheet, radio options)
13. Settings — Split Tunneling (app list with toggles)
14. Profiles — List (presets + custom)
15. Profiles — Detail (server, protocol, split tunnel config)
16. Account — Profile (subscription, referral)
17. Subscription — Plans (Free vs Pro comparison)
18. Onboarding — Welcome
19. Onboarding — Permissions

### 3.5 Rules

- NO dark theme. White base only.
- NO emojis anywhere in the production app. Use icons from a consistent icon set (Lucide, Material Symbols, or custom SVG).
- All text in Brazilian Portuguese.
- No technical jargon in user-facing copy.
- Gold is precious. Use only for: CTAs, active nav tab, toggle on-state, stat numbers on dark blocks, and connection progress. Everything else is grayscale.
- Inter font only. JetBrains Mono only for numerical data.
- Pill-shaped buttons always (border-radius 100dp).
- Cards have no elevation/shadow. Use border only.
- The map is the centerpiece of the home screen. It must feel premium.

---

## Implementation Priority

### Phase 1 — Retheme and Critical (1-2 weeks)
- Pearl and Precision color system and typography in Compose theme
- Home screen redesign with map, IP display, stats
- 0-100% connection animation
- Kill Switch (VpnService always-on)
- Add lat/lng to servers table and API response
- 4-tab bottom navigation

### Phase 2 — Server UX (1 week)
- City-level server selection with expandable list
- Search bar and filter tabs
- Favorites (new table + API + star UI)
- Recents (track in usage_logs + API)
- Server ping from app

### Phase 3 — Power Features (2 weeks)
- Split Tunneling UI and VpnService integration
- Protocol picker bottom sheet
- MultiHop entry/exit picker UI
- Shield dashboard with daily/weekly stats
- Blocked domains list
- Connection profiles (new table + API + screens)
- Pause VPN with timer

### Phase 4 — Growth (1 week)
- Onboarding flow (welcome + permissions)
- Account screen with subscription status
- Subscription plan comparison (Free vs Pro)
- Referral system (new table + API + UI)
- Auto-connect on unsafe WiFi
- LAN Discovery toggle

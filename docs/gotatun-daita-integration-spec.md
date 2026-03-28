# ESCUDO VPN — GotaTun + DAITA Integration Specification
## Post-Launch Engine Upgrade: From BoringTun to GotaTun
### Version 1.0 — March 2026

---

## EXECUTIVE SUMMARY

This specification defines the post-launch upgrade of Escudo VPN's client-side WireGuard implementation from Cloudflare's BoringTun to Mullvad's GotaTun — a superior Rust-based WireGuard engine with DAITA (Defense Against AI-guided Traffic Analysis), Multihop support, and proven Android stability improvements.

This is a CLIENT-SIDE ONLY change. The server infrastructure stays exactly the same (kernel WireGuard). The protocol between client and server is identical — WireGuard talks to WireGuard regardless of implementation. No server redeployment, no database changes, no API changes. The upgrade is an app update that users install from the app store.

Key outcome: Escudo becomes the SECOND VPN in the world (after Mullvad) to ship GotaTun with DAITA, and the ONLY VPN to combine it with residential IP streaming, DNS security suite, and anonymous PIX payment.

---

## WHAT IS GOTATUN

GotaTun is Mullvad's fork of Cloudflare's BoringTun project — a userspace WireGuard implementation written entirely in Rust.

- Source: github.com/mullvad/gotatun
- License: Mozilla Public License 2.0 (MPL-2.0) as of March 5, 2026. Contributions prior to that date remain under BSD 3-Clause.
- License implications for Escudo: MPL-2.0 allows commercial use and proprietary code. File-level copyleft means: if you MODIFY a Mullvad source file, that specific file must be released under MPL-2.0. Your OWN new files (IP Guardian integration, residential proxy routing, Escudo-specific features) remain proprietary. This is standard for commercial use of MPL-licensed software (Firefox, LibreOffice, and Syncthing all use MPL-2.0).
- GitHub stats: ~1,200 stars, 31 forks, actively maintained by Mullvad's core team
- Current version: v0.4.1 (March 2026) — includes Assured Security audit report
- Platforms supported: Linux (x86_64, aarch64, armv7), macOS, Windows, iOS, Android
- Audit status: Independently audited by Assured Security Consultants (Jan-Feb 2026). No major vulnerabilities found. Two low-severity issues fixed in v0.4.0.

### Why GotaTun over raw BoringTun

| Capability | BoringTun (current) | GotaTun (upgrade) |
|---|---|---|
| Language | Rust | Rust |
| WireGuard protocol | Full | Full |
| Android stability | Crash rate ~0.40% | Crash rate 0.01% (40x improvement) |
| DAITA (anti-AI traffic analysis) | No | Yes, built-in |
| Multihop support | No | Yes, built-in |
| Zero-copy memory strategies | No | Yes |
| Safe multi-threading | Basic | Advanced |
| Independent security audit | Cloudflare's original only | Assured Security 2026 + Cure53 on infrastructure |
| Battery efficiency (mobile) | Standard | Improved (user-reported) |
| Active maintenance | Cloudflare (low activity) | Mullvad (very active, weekly commits) |

BoringTun is essentially unmaintained by Cloudflare — their last significant commit was years ago. GotaTun is actively developed with weekly commits, security fixes, and feature additions. Staying on BoringTun means accumulating technical debt. Moving to GotaTun aligns Escudo with the most actively maintained WireGuard Rust implementation in the world.

---

## WHAT IS DAITA

DAITA — Defense Against AI-guided Traffic Analysis — is Mullvad's proprietary privacy feature that prevents traffic analysis attacks on encrypted VPN tunnels.

### The problem DAITA solves

Even though WireGuard encrypts all tunnel traffic, an observer (ISP, government, network administrator) can still analyze the PATTERNS of encrypted traffic to determine what a user is doing:

- Packet size analysis: A video stream produces different packet sizes than web browsing, which produces different sizes than file downloads. An AI model trained on traffic patterns can identify "this encrypted tunnel is streaming Netflix" with 80-95% accuracy without decrypting anything.
- Timing analysis: The timing between packets reveals activity patterns. Interactive browsing has bursty, irregular timing. Video streaming has steady, regular timing. VoIP has very regular small packets.
- Volume analysis: Total data volume over time reveals whether someone is streaming (high sustained throughput) vs browsing (low bursty throughput) vs idle.

Nation-state adversaries (NSA, GCHQ, China's GFW) and sophisticated ISPs actively use these techniques. Academic research has demonstrated 90%+ accuracy in classifying encrypted VPN traffic by website visited.

### How DAITA works

DAITA applies three countermeasures simultaneously:

1. Constant packet sizes: All packets are padded to uniform sizes. Whether you're loading a text email or streaming 4K video, every packet leaving the tunnel is the same size. This eliminates packet-size-based classification entirely.

2. Dummy traffic injection (chaff): DAITA injects fake packets that look identical to real traffic. An observer sees a constant stream of uniform-sized packets and cannot distinguish real data from noise. This masks timing patterns and volume analysis.

3. Traffic pattern machines: DAITA uses "machines" — algorithmic models that generate realistic-looking traffic patterns. Even when the user is idle, the tunnel appears to be carrying normal traffic. This prevents idle/active state detection.

### Performance impact

DAITA adds overhead:
- Bandwidth: 10-30% increase due to padding and dummy traffic
- Latency: Minimal (sub-millisecond)
- Speed: 10-15% reduction in effective throughput
- Battery: Slight increase due to continuous traffic generation

This is acceptable for privacy-focused users but undesirable for streaming-focused users who want maximum speed.

### Implementation in Escudo

DAITA will be offered as an OPTIONAL toggle in the Escudo app:

| Tier | DAITA available | Default state | Label in app |
|---|---|---|---|
| Free | No | — | — |
| Escudo (R$8) | Yes | OFF | "Privacidade Maxima" |
| Pro (R$35) | Yes | OFF | "Privacidade Maxima" |

When DAITA is OFF: standard WireGuard tunnel, maximum speed, residential IP streaming works normally.

When DAITA is ON: padded packets, dummy traffic, traffic pattern machines active. Speed reduced ~10-15%. Streaming still works but at slightly lower quality. The app shows: "Privacidade Maxima: Seus padroes de trafego estao protegidos contra analise de IA. A velocidade pode ser reduzida."

This gives users the CHOICE. Privacy maximalists toggle it on. Netflix streamers leave it off. Escudo is the only VPN besides Mullvad offering this choice at any price point.

---

## TECHNICAL INTEGRATION PLAN

### Current Escudo client architecture

```
Escudo App (Kotlin/Swift)
  -> escudo-client (Rust library via C FFI)
       -> boringtun (WireGuard implementation)
            -> Kernel tun/tap device
```

### Target architecture after GotaTun integration

```
Escudo App (Kotlin/Swift)
  -> escudo-client (Rust library via C FFI)
       -> gotatun (WireGuard + DAITA + Multihop)
            |-- DAITA module (packet padding, chaff, machines)
            |-- Multihop module (double-server routing)
            -> Kernel tun/tap device
```

### Integration steps

Step 1: Fork GotaTun (1 day)
- Fork github.com/mullvad/gotatun to Escudo org
- Pin to v0.4.1 (latest audited version)
- Study the crate structure: gotatun is a Rust library crate with an optional CLI binary

Step 2: Replace boringtun dependency in escudo-client (2-3 days)
- In escudo-client's Cargo.toml, replace boringtun dependency with local path to forked gotatun
- The API is similar but not identical — GotaTun has a reworked device API (see v0.2.0 changelog)
- Update the tunnel creation code in escudo-client to use GotaTun's API
- Key functions: Tunn::new(), Tunn::decapsulate(), Tunn::encapsulate()
- GotaTun uses the same WireGuard UAPI protocol, so wg and wg-quick still work

Step 3: Wire DAITA toggle (1-2 days)
- DAITA is controlled via GotaTun's configuration API
- Add a boolean daita_enabled field to the escudo-client config
- When true: enable DAITA's padding, chaff, and machines modules
- When false: standard WireGuard behavior (no overhead)
- Expose this as a C FFI function: escudo_set_daita_enabled(bool)
- App layer (Kotlin/Swift) calls this when user toggles "Privacidade Maxima"

Step 4: Wire Multihop (optional, 2-3 days)
- GotaTun supports routing traffic through two WireGuard servers sequentially
- First hop: user -> entry server (e.g., Sao Paulo)
- Second hop: entry server -> exit server (e.g., Miami)
- This means an adversary who compromises one server still can't see both the user's real IP and their destination
- Expose as: escudo_set_multihop(entry_server, exit_server)
- Available on Pro tier only

Step 5: Android-specific integration (2-3 days)
- GotaTun has "first-class Android support" — this is one of its primary improvements over BoringTun
- On Android: use GotaTun's JNI bindings or Rust-to-Kotlin via UniFFI
- The VpnService API stays the same — you just change which library handles the tunnel
- Test with Android's Always-On VPN and kill switch enforcement

Step 6: iOS-specific integration (2-3 days)
- GotaTun supports iOS via the NetworkExtension framework
- NEPacketTunnelProvider remains the entry point
- Replace the BoringTun tunnel handler with GotaTun's
- Note: Mullvad hasn't shipped GotaTun on iOS yet (planned for 2026) — Escudo could potentially beat them

Step 7: Testing (3-5 days)
- VPN tunnel connectivity (does it connect, does traffic flow)
- IP leak test with DAITA on and off
- DNS leak test with DAITA on and off
- Speed test comparison: BoringTun vs GotaTun vs GotaTun+DAITA
- Battery drain comparison on Android (24-hour test with screen off)
- Crash monitoring (GotaTun's key improvement: 0.01% crash rate vs 0.40%)
- Streaming test through residential IPs with DAITA on (verify Netflix still works)
- Kill switch test with GotaTun (force-kill process, verify no leak)

Total estimated effort: 2-3 weeks for one Rust developer

### What does NOT change

- Server-side infrastructure (kernel WireGuard stays)
- WireGuard protocol version and compatibility
- DNS resolver (Hickory DNS stays)
- Residential IP proxy chain (IP Guardian stays)
- Database schema
- API endpoints
- Payment integration
- User accounts
- Server provisioning pipeline

The entire change is contained within the client app binary. Ship it as an app update.

---

## ANONYMOUS ACCOUNT MODEL SPECIFICATION

Alongside the GotaTun upgrade, implement the Mullvad-style anonymous account system.

### Account creation flow

1. User downloads Escudo app from Play Store / App Store
2. App generates a random 16-digit account number locally on the device
   - Format: XXXX-XXXX-XXXX-XXXX
   - Generated using cryptographically secure random number generator
   - No server call needed for generation
3. User sees: "Sua conta Escudo: 4829-7361-0584-2917"
4. App prompts: "Salve este numero! E sua unica forma de acessar sua conta."
5. Account number is sent to server API to register (just the number, no personal data)
6. Server creates account record: account_number, created_at, tier='free', status='active'

### Payment flow (PIX via GetMoons)

1. User taps "Upgrade to Escudo (R$8/mes)" or "Upgrade to Pro (R$35/mes)"
2. App generates a PIX payment request via GetMoons API
   - Amount: R$8 or R$35
   - Reference: account number (4829-7361-0584-2917)
   - Customer sees PIX QR code or copy-paste code
3. Customer pays via their bank app (Nubank, Itau, Bradesco, etc.)
4. GetMoons confirms payment -> sends webhook to Escudo API
5. Escudo API activates the tier for that account number
6. App refreshes and shows: "Escudo ativo! VPN + Protecao + Bloqueio de anuncios"

### Optional email addition

After account creation, user can optionally add an email:
- "Adicionar email para recuperacao e monitoramento de vazamentos (opcional)"
- If added: enables dark web monitoring, breach alerts, account recovery
- If not added: fully anonymous account, no recovery possible if number is lost

### Multi-device login

1. User installs Escudo on second device
2. App shows: "Ja tem conta? Digite seu numero"
3. User enters: 4829-7361-0584-2917
4. App authenticates against server API
5. New WireGuard keypair generated for this device
6. Device registered under the account
7. Both devices now connected to Escudo

### Database schema

```sql
CREATE TABLE accounts (
    account_number VARCHAR(19) PRIMARY KEY,  -- XXXX-XXXX-XXXX-XXXX
    email VARCHAR(255) NULL,                 -- optional
    tier VARCHAR(20) DEFAULT 'free',         -- free, escudo, pro
    status VARCHAR(20) DEFAULT 'active',
    dedicated_ip_id UUID NULL,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    paid_until TIMESTAMPTZ NULL,
    devices_count INTEGER DEFAULT 0
);
-- No name, no CPF, no phone number, no address.
-- If email is NULL, account is fully anonymous.
```

---

## MARKETING POSITIONING

### Launch announcement (post-upgrade)

Headline (PT-BR): "Escudo agora usa o mesmo motor de privacidade do Mullvad — o VPN mais respeitado do mundo"

Headline (EN): "Escudo now runs on GotaTun — the same privacy engine trusted by Mullvad VPN"

Key messages:
- GotaTun is Mullvad's WireGuard implementation in Rust — audited, open-source, zero crashes
- DAITA (Privacidade Maxima) protects against AI traffic analysis — a feature only Mullvad and Escudo offer
- Anonymous account with PIX payment — no email, no CPF, no personal data required
- All this PLUS residential IP streaming, ad blocking, parental controls — features Mullvad doesn't offer

Tagline: "Privacidade nivel Mullvad. Streaming nivel Netflix. Preco brasileiro."

### Competitive comparison after upgrade

| Feature | Mullvad | NordVPN | Escudo (post-upgrade) |
|---|---|---|---|
| WireGuard in Rust (GotaTun) | Yes (creator) | No (NordLynx/Go) | Yes (fork) |
| DAITA anti-AI analysis | Yes | No | Yes |
| Anonymous accounts | Yes (16-digit) | No (email required) | Yes (16-digit) |
| Cash/crypto payment | Yes | Crypto only | PIX + Crypto |
| Residential IP streaming | No | No | Yes |
| DNS ad blocking | Basic | Threat Protection | Full Hagezi 400K+ |
| Parental controls | No | No | Yes (DNS-level) |
| Dark web monitoring | No | Plus tier only | R$8 tier |
| Streaming unlock | No (deliberately) | Yes (datacenter IPs) | Yes (residential IPs) |
| Price | 5 EUR/month flat | $3.39-12.99/month | R$8/month (~$1.50) |
| Devices | 5 | 10 | 5 (free), 5 (R$8), 10 (Pro) |

Escudo becomes the ONLY product that combines Mullvad's privacy technology with NordVPN's consumer features at Brazilian pricing. This is a category of one.

---

## TIMELINE

| Phase | Task | Duration | Dependencies |
|---|---|---|---|
| LAUNCH | Ship Escudo with BoringTun (current) | NOW | None — launch doesn't wait for GotaTun |
| Week 2-3 post-launch | Fork GotaTun, study codebase | 1 week | None |
| Week 3-4 | Replace BoringTun with GotaTun in escudo-client | 1 week | Fork complete |
| Week 4-5 | Wire DAITA toggle + Multihop | 1 week | GotaTun integrated |
| Week 5-6 | Android + iOS integration + testing | 2 weeks | All above |
| Week 7 | App update release with GotaTun + DAITA + anonymous accounts | 1 day | Testing complete |
| Week 8+ | Marketing push: "Now powered by GotaTun" | Ongoing | Release shipped |

Total: 5-7 weeks post-launch for the full upgrade.

Launch NOW with BoringTun. It works. It's tested. It passed your audit. Then upgrade to GotaTun as the first major post-launch release. Users see "Escudo 2.0 — Agora com Privacidade Maxima" and the anonymous account option. Press coverage follows because you're the second VPN in the world using GotaTun and the first to combine it with residential IPs.

---

## RISK ASSESSMENT

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| GotaTun API changes break integration | Low | Medium | Pin to v0.4.1, don't track main branch |
| MPL-2.0 license compliance issues | Low | High | Keep modified Mullvad files separate, release under MPL-2.0 |
| DAITA performance worse than expected | Medium | Low | DAITA is optional toggle, default OFF |
| Mullvad deprecates GotaTun | Very Low | High | Fork is independent, BSD/MPL licensed |
| Netflix blocks DAITA traffic patterns | Low | Medium | DAITA is optional, disable for streaming |
| Apple/Google reject app update | Low | Medium | GotaTun already on Android Play Store via Mullvad |

---

## CONCLUSION

GotaTun + DAITA + anonymous accounts transform Escudo from "a Brazilian VPN" into "the most privacy-advanced VPN in Latin America." The engineering effort is 5-7 weeks post-launch. The cost is zero (open-source). The competitive impact is permanent — you'll be running the same tunnel engine as the most respected VPN in the world, with features they deliberately don't offer, at a price point they'll never match.

Launch first. Upgrade second. Announce loud.

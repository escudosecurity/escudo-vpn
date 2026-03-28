# Escudo Site/App Handoff

Date: 2026-03-28

This document is the current handoff for the developer working on the website, plans, checkout, and site-to-app connection. It reflects the real state of the product now, not the earlier simplified VPN-only model.

## 1. Product State Now

Escudo is no longer just:
- free VPN
- pro VPN
- dedicated VPN

It is now becoming a broader product with these layers:
- Standard VPN
- Residential routes
- Family / parental supervision
- Dedicated premium routing

The Android app is being actively refined on the restored white pearl branch. Backend ops, telemetry, launch controls, and family foundations are already live.

## 2. What Is Already Live

### Backend / Ops
- launch controls table and APIs
- invite code system for tier upgrades
- VPN session ledger
- journey event telemetry
- node metrics collector
- health scoring and lifecycle states
- admin ops dashboard
- alerts / overview APIs

### Auth / Account
- anonymous account creation
- 16-digit account code login
- email register/login still exists
- QR pairing backend endpoints exist

### Family / Parental
- child profile creation
- child access codes
- child-device linking
- per-child policies
- schedules / bedtime windows
- parental events
- child device policy fetch

### Routing / Tiers
- real routing lanes:
  - `free`
  - `escudo`
  - `pro`
  - `dedicated`
- shared residential routing is live
- dedicated residential routing is live

## 3. Important Product Change

The site developer should not think in the old model of:
- "app = anonymous code only"
- "site = normal VPN pricing page"

That is outdated.

Current reality:
- the app supports account-code onboarding
- the backend supports email auth too
- paid plans, family, residential, and premium product shaping all now matter

## 4. Current Plan Logic

Internal backend routing tiers today are:
- `free`
- `escudo`
- `pro`
- `dedicated`

But external product packaging should likely be presented more clearly as:
- Free
- Escudo
- Residential
- Family
- Dedicated

Important mapping note:
- `Residential` is a product offer, but likely maps onto `escudo` or `pro` depending on commercial choice
- `Family` is a product offer, and currently should map onto `pro` lane unless backend billing/tier tables are expanded
- `Dedicated` maps to `dedicated`

## 5. Site Developer Should Build Around This

The site/payment flow should support both:

### Option A: Paid flow with email
- user picks a plan
- enters email
- pays
- account is activated
- app login can use email

### Option B: Code-first flow
- user gets or already has a 16-digit code
- user upgrades that account through checkout
- webhook or post-payment flow upgrades that existing account

This means email is useful, but email should not be treated as the only valid account model.

## 6. My Recommendation On Email

Do not force the whole product into email-only thinking.

The right model now is:
- free users may start code-first
- paid users may use email-first
- existing code-based users must be able to upgrade without breaking their account path

So the correct commercial/account model is:
- support email
- keep 16-digit code support
- link them cleanly later where needed

Email is valuable for:
- receipts
- renewal reminders
- recovery
- support

But code-based onboarding is already a real product behavior and should not be thrown away.

## 7. What The Site Needs Next

The website developer should build:

1. Pricing / plans page
2. Clear mapping from site plans to backend tiers
3. Checkout flow
4. Success page that tells user how to access the app
5. Account path that supports:
   - new paid email account
   - existing code account upgrade
6. App download page
7. Plan messaging for:
   - Standard
   - Residential US / UK / EU
   - Family supervision
   - Dedicated

## 8. What The Website Should NOT Assume

Do not assume:
- all users must start with email
- family is not real yet
- residential is just a hidden internal thing
- app account code is temporary or irrelevant
- there is only one premium lane

## 9. Current Android App Direction

Current Android direction after recent cleanup:
- white pearl branch restored
- real world map on home
- residential tab separated
- standard servers separated
- account code moved into account section
- account has copy-code and pairing entry points

Still missing:
- real QR scanner flow
- final account/device-link polish
- final premium residential polish
- Windows app

## 10. Practical Handoff For The Site Dev

The site developer should work from this operating assumption:

"Escudo supports both email-based paid accounts and 16-digit code-based app accounts. The site should sell plans cleanly, activate entitlements correctly, and hand the user into the app without breaking either path."

That is the correct model now.

## 11. Immediate Decision Needed

Before the site is finalized, decide this explicitly:

### Recommended
- allow paid users to start with email
- allow existing 16-digit users to upgrade the same account
- do not force all users into one model

That gives the most flexibility and avoids breaking the app's current account behavior.

# EscudoVPN Staging Sign-Off Template

Release candidate:
Date:
Environment:
Release owner:
Security reviewer:

## Scope

- Android APK
- Android AAB
- Backend API
- Gateway
- DNS service

## Artifact Record

- APK path:
- AAB path:
- APK SHA256:
- AAB SHA256:
- Android versionName:
- Android versionCode:
- API commit:
- Android commit:

## Pre-Release Gates

- [ ] `cargo check --workspace` passed
- [ ] `cargo audit` passed
- [ ] Android release signing env vars were present
- [ ] unsigned release build path failed closed
- [ ] signed `assembleRelease` succeeded
- [ ] signed `bundleRelease` succeeded
- [ ] TLS verified for `api.escudovpn.com`
- [ ] SPKI pins in release config match current production cert chain strategy

## Synthetic Test Results

- [ ] API smoke test passed
- [ ] connect-cycle test passed
- [ ] auth rate limit checked
- [ ] `/api/v1/ws/stats` requires auth
- [ ] `/api/v1/stats/dns` is user-scoped

Command outputs or links:

## Android Manual Matrix

- [ ] Pixel current Android
- [ ] Samsung current Android
- [ ] Xiaomi or equivalent aggressive OEM
- [ ] older Android device

Notes:

## Resilience Checks

- [ ] API restart handled
- [ ] gateway restart handled
- [ ] DB outage failure mode observed
- [ ] DNS stats degradation observed
- [ ] rollback steps documented

Notes:

## Risks

- Open risks:
- Accepted risks:
- Deferred post-launch work:

## Verdict

- [ ] Approve release
- [ ] Approve limited private distribution only
- [ ] Reject release

Rationale:

## Sign-Off

Release owner:

Security reviewer:

Operations reviewer:

# Escudo VPN Ops Control Plane Design
## Routing, Health, Capacity, Devices, Abuse, and Automation
### Version 1.0 — March 2026

---

## Executive Summary

This document defines the operational control plane for Escudo VPN after the March 2026 benchmarking work.

The main goal is simple:

- keep user experience stable
- scale without manual node babysitting
- protect margins on the free tier
- preserve premium quality for paid users
- stop abuse without collecting invasive personal data

The system should make routing and capacity decisions automatically based on:

- node health
- node capacity
- account plan
- device behavior
- dedicated-route requirements
- abuse/fraud signals

The operational model is based on three service classes:

- `Free`
- `Medium`
- `Power`

This document does not define final customer packaging or marketing copy. It defines the backend operating model that plan packaging will sit on top of.

---

## 1. Benchmark Baseline

The March 2026 live phone-under-load tests established a usable class ladder with the same `464` mixed-session load model.

Measured examples:

- Brazil `Medium`: about `6.47 Mbps`
- Brazil `Power`: about `14.01 Mbps`
- Germany `Medium`: about `9.97 Mbps`
- Germany `Power`: about `21.29 Mbps`

Operational conclusions:

- the `Free` class is viable for capped, best-available usage
- the `Medium` class is strong enough for the mass paid tier
- the `Power` class is the premium/high-sensitivity tier

Initial planning baseline:

- `350 assigned users per node`

Important:

- this means assigned users attached to a node or pool
- this does not mean `350` heavy active streamers at the same second

This baseline is the starting allocation number. It must be adjusted from live telemetry after launch.

---

## 2. Core Principles

The control plane should enforce these principles:

1. Users should be routed by health and fit, not by static node labels alone.
2. New sessions should avoid degraded nodes automatically.
3. Existing sessions should not be disrupted unless a node is actually broken.
4. Free users should consume the cheapest healthy capacity that still feels good.
5. Paid users should receive better priority and better classes automatically.
6. Dedicated or residential-sensitive users must never be silently downgraded to an unsuitable path.
7. Abuse prevention must use minimal, privacy-safe identifiers instead of invasive tracking.
8. Node operations must be automated by default and surfaced in an internal dashboard.

---

## 3. Service Classes

Escudo operates three backend service classes.

### 3.1 Free

Purpose:

- customer acquisition
- best-available low-cost capacity
- capped usage

Characteristics:

- lowest routing priority
- cheapest healthy node pool
- strongest oversubscription tolerance
- hard monthly cap

### 3.2 Medium

Purpose:

- mass paid tier
- better consistency
- multi-device default for most paying users

Characteristics:

- higher priority than `Free`
- better per-user quality
- lower oversubscription than `Free`

### 3.3 Power

Purpose:

- premium users
- heavy users
- dedicated/residential-sensitive use cases

Characteristics:

- highest routing priority
- lowest oversubscription
- reserved capacity for sensitive or high-value sessions
- supports dedicated-route pinning

---

## 4. Node Lifecycle States

Every node must always have a machine-readable lifecycle state.

States:

- `provisioning`
- `healthy`
- `warm`
- `degraded`
- `draining`
- `blocked`
- `retiring`

Meanings:

- `provisioning`: node exists but is not eligible for user assignment
- `healthy`: normal routing weight, accepts new users
- `warm`: slightly reduced weight, still accepts new users
- `degraded`: do not assign new users unless no better option exists
- `draining`: keep existing users, no new assignments
- `blocked`: no new users and reconnects should avoid the node
- `retiring`: scheduled removal from fleet

These states are controlled by automation, not by ad hoc human judgment.

---

## 5. Health Model

Every node gets a rolling `health_score` from `0` to `100`.

Start from `100` and subtract penalties.

### 5.1 Inputs

Collected every `1 minute`:

- CPU usage
- RAM usage
- NIC inbound Mbps
- NIC outbound Mbps
- active WireGuard peers
- assigned user count
- connect success rate
- median connect time
- disconnect rate
- handshake freshness
- error count

Collected every `5-10 minutes` via probes:

- browse probe success
- browse probe latency
- video-chunk probe success
- video-chunk probe throughput
- control-plane connect latency

### 5.2 Suggested Score Model

Example penalties:

- connect success below `98%`: subtract `10-40`
- median connect time over `3s`: subtract `5-20`
- browse probe failure: subtract `20`
- video probe failure: subtract `20`
- CPU over `80%`: subtract `10`
- CPU over `90%`: subtract `20`
- RAM over `85%`: subtract `10`
- sustained NIC over `75%`: subtract `10`
- sustained NIC over `90%`: subtract `20`
- disconnect spike: subtract `10-25`
- packet-loss or retry spike: subtract `10-25`

### 5.3 Score Bands

- `85-100`: `healthy`
- `70-84`: `warm`
- `50-69`: `degraded`
- `<50`: `blocked`

The exact weights can be tuned later. The important part is that they are deterministic and explainable.

---

## 6. Capacity Model

Node health and capacity are different things. Both must be tracked.

Each node should store:

- `assigned_user_cap`
- `active_session_soft_cap`
- `active_session_hard_cap`
- `throughput_soft_cap_mbps`
- `throughput_hard_cap_mbps`

Initial baseline:

- `assigned_user_cap = 350`

Recommended default thresholds:

- soft cap at about `80%`
- hard cap at about `100%`

Example for a `350` baseline:

- soft cap: `280`
- hard cap: `350`

Capacity enforcement rules:

- under soft cap: normal weight
- over soft cap: reduce weight
- over hard cap: no new assignments
- if node health also drops: move to `degraded` or `blocked`

This model must consider both user count and quality metrics. A node can be under count cap and still unhealthy.

---

## 7. Routing and Allocation

The allocator is the runtime brain of the system.

### 7.1 Allocation Flow

On every connect request:

1. identify account plan
2. identify connecting device
3. determine allowed classes for that account and device
4. determine preferred region candidates
5. filter out ineligible nodes:
   - wrong class
   - blocked
   - hard-capped
   - incompatible dedicated requirements
6. rank remaining nodes by:
   - health score
   - load score
   - geography fit
   - plan priority
   - dedicated-route suitability
   - recent success for that region
7. assign best node

### 7.2 Region Fit

Region fit should be based on:

- signup country
- recent login country
- selected country
- best measured regional path

The system should prefer "best healthy route" over fixed static assumptions.

### 7.3 Reconnect Behavior

For a degraded node:

- existing sessions should usually remain connected
- reconnects should prefer a healthier node

For a blocked node:

- new sessions must not land there
- reconnects must avoid it

This avoids unnecessary user disruption while still protecting quality.

---

## 8. Account and Device Model

This is one of the most important parts of the ops design.

Plan entitlements should be evaluated at both:

- account level
- device level

### 8.1 Privacy-Safe Device Identity

Use a minimal app-scoped identifier:

- `device_install_id`

This should be:

- randomly generated on first app install
- unique to Escudo
- reset on reinstall
- not derived from invasive hardware fingerprinting

This is sufficient for:

- registered device counts
- active device counts
- free-account farming detection

It should not be used for advertising or external profiling.

### 8.2 Device Fields

Per device, store:

- device id
- account id
- `device_install_id`
- device nickname
- platform
- first seen
- last seen
- app version
- active session count
- usage bucket
- preferred class
- `dedicated_required`
- `sensitive_route`

### 8.3 Registered vs Active Devices

Do not use a single device limit.

Track both:

- `registered_devices_limit`
- `active_devices_limit`

This allows a user to keep multiple installations without enabling unlimited simultaneous use.

Example working defaults:

- `Free`: `1 registered / 1 active`
- entry paid plan: `3 registered / 2-3 active`
- middle paid plan: `5 registered / 3-5 active`
- premium plan: `10 registered / 5-10 active`

Final commercial numbers can change later. The control-plane model supports either stricter or more generous packaging.

### 8.4 Per-Device Class Assignment

The system should be able to route different devices on the same account into different classes.

Example:

- premium phone: `Power`
- frequently used laptop: `Power` or `Medium`
- rarely used tablet: `Medium`
- idle TV: lower-priority eligible pool

This is a major scaling lever because it avoids wasting expensive premium capacity on low-value or idle endpoints.

Per-device pool changes should usually happen on next connect, not by forcing a live session move.

---

## 9. Dedicated and Sensitive Routes

Some devices or users require special routing:

- banking-sensitive traffic
- residential or ISP-style exits
- dedicated IP paths
- compliance-sensitive region paths

These should carry explicit policy flags:

- `dedicated_required`
- `sensitive_route`

Rules:

- never silently downgrade these sessions to `Free`
- never route them onto a node class that cannot satisfy the requirement
- if the dedicated route is unhealthy, alert operations and fail safely

Sensitive routes must be pinned by policy, not guessed at runtime.

---

## 10. Free Tier Controls

The free tier can only be sustainable if it is controlled tightly.

Recommended initial operating model:

- hard monthly cap
- lowest routing priority
- best healthy `Free` pool only
- single-device default

The free tier should be judged by:

- whether it feels usable
- whether it converts
- whether it stays inside cost guardrails

It should not be designed to mimic the burn rate of giant incumbents.

---

## 11. Abuse and Fraud Control

This layer is required for launch stability.

### 11.1 Abuse Signals

Track abuse indicators such as:

- many free signups from one device
- many free signups from one IP or subnet in short time
- repeated cap exhaustion followed by fresh registrations
- too many simultaneous devices on one account
- impossible country switching
- synchronized usage patterns consistent with resale or sharing

### 11.2 Minimal Data Principle

Use only data required for operations:

- signup IP
- login IP
- coarse country
- `device_install_id`
- account age
- plan
- usage patterns

Do not use:

- cross-app device fingerprinting
- precise location history
- contact data unrelated to service operation

### 11.3 Abuse Responses

Graduated responses:

- reduce priority
- rate limit registration
- block new free account creation from suspicious device or subnet
- limit concurrency
- require verification
- queue manual review for edge cases

The system should not overreact to normal household use.

---

## 12. Usage Classification

The system does not need to know exactly what content a user is accessing.

It does need to know the shape of usage.

Classify users and devices by:

- session duration
- total bytes
- average throughput
- peak throughput
- reconnect frequency
- active-device concurrency

Derived buckets:

- `light`
- `normal`
- `heavy`
- `sensitive`

These buckets support:

- per-device routing
- fair-use enforcement
- premium protection
- anomaly detection

---

## 13. Automation Cadence

Use three loops.

### 13.1 Fast Health Loop

Every `1 minute`:

- collect node metrics
- collect connect success and connect latency
- recompute health score
- change node lifecycle state if needed

### 13.2 Quality Probe Loop

Every `5-10 minutes`:

- run browse probe
- run video-chunk probe
- run control-plane connect probe

### 13.3 Capacity and Rebalance Loop

Every `5 minutes`:

- recompute node weights
- reduce weight for warm nodes
- mark degraded nodes unavailable for new assignments
- trigger draining or scale-up actions when thresholds are crossed

For severe incidents, the system should not wait for the next slow loop. It should react immediately.

---

## 14. Operational Actions

The control plane should take these actions automatically.

### 14.1 Lower Weight

When:

- node is `warm`
- near soft cap

Effect:

- still eligible
- receives fewer new sessions

### 14.2 Drain

When:

- node is `degraded`
- quality is dropping

Effect:

- keep existing sessions if still stable
- no new assignments

### 14.3 Block

When:

- node is `blocked`
- routing is broken
- connect failures spike severely

Effect:

- no new sessions
- reconnects go elsewhere

### 14.4 Scale Trigger

When:

- region or class keeps living near cap
- multiple nodes in same class are warm or degraded

Effect:

- create scale-up event
- or page operations if auto-provisioning is not yet enabled

---

## 15. Dashboard Requirements

One internal ops dashboard is required.

### 15.1 Global View

Show:

- active users by plan
- assigned users by class
- users by country
- node health distribution
- total throughput
- free cap consumption
- conversion funnel

### 15.2 Node Table

For each node:

- node id
- name
- class
- country
- provider
- health score
- lifecycle state
- assigned users
- active sessions
- connect success
- browse probe status
- video probe status
- inbound/outbound Mbps
- cap status

### 15.3 Device and Abuse View

Show:

- accounts with too many active devices
- device ids linked to many accounts
- free-account clusters by subnet or device
- accounts with suspicious country switching
- dedicated-route failures

### 15.4 Alerts

Show:

- degraded nodes
- blocked nodes
- routing misconfigurations
- free pool saturation
- dedicated route failures
- abuse spikes

---

## 16. Data Model

Suggested core relational tables.

### 16.1 accounts

- id
- email or account number
- plan_id
- status
- billing_country
- signup_ip
- signup_country
- created_at
- abuse_score

### 16.2 devices

- id
- account_id
- device_install_id
- nickname
- platform
- first_seen_at
- last_seen_at
- current_active_sessions
- usage_bucket
- preferred_class
- dedicated_required
- sensitive_route

### 16.3 nodes

- id
- name
- country
- city
- provider
- class
- monthly_cost_usd
- assigned_user_cap
- active_session_cap
- lifecycle_state

### 16.4 node_metrics

- node_id
- ts
- cpu_pct
- ram_pct
- nic_in_mbps
- nic_out_mbps
- active_sessions
- connect_success_pct
- median_connect_ms
- health_score
- health_state

### 16.5 sessions

- id
- account_id
- device_id
- node_id
- class
- started_at
- ended_at
- bytes_in
- bytes_out
- connect_country

### 16.6 probes

- node_id
- ts
- probe_type
- success
- latency_ms
- throughput_mbps

---

## 17. Services and Jobs

The core control plane should be implemented in Rust.

### 17.1 allocator-service

Responsibility:

- choose node on connect request

### 17.2 health-service

Responsibility:

- ingest metrics and probes
- compute node score and lifecycle state

### 17.3 policy-service

Responsibility:

- plan entitlements
- device rules
- free cap
- abuse score
- dedicated-route eligibility

### 17.4 rebalancer-service

Responsibility:

- update routing weights
- drain or block nodes
- trigger scale events

### 17.5 alert-service

Responsibility:

- send alerts to dashboard and message channels

### 17.6 usage-classifier

Responsibility:

- classify accounts and devices into usage buckets

If jobs start as scheduled workers, they should still be moved toward durable services over time.

---

## 18. Initial Policy Defaults

Good starting defaults for launch:

- class model: `Free`, `Medium`, `Power`
- baseline user cap per node: `350`
- soft cap around `80%`
- hard cap around `100%`
- health loop every `1 minute`
- probes every `10 minutes`
- rebalance every `5 minutes`

Current working product assumptions:

- `Free`: one device, hard monthly cap
- entry paid: default `Medium`
- mid paid: better priority, mostly `Medium`
- premium paid: default `Power`

Commercial packaging can change later without changing the underlying control-plane architecture.

---

## 19. What This Automates

Once implemented, the control plane should automatically handle:

- routing new users away from bad nodes
- keeping connected users stable where possible
- protecting premium capacity from cheap-tier abuse
- downgrading low-value device placements when safe
- pinning sensitive users to correct routes
- limiting free-tier farming
- surfacing real-time ops issues to a dashboard

This is what allows Escudo to scale without hand-managing every node.

---

## 20. Implementation Order

Recommended build sequence:

1. node metrics collection
2. probe runner
3. node health scoring
4. allocator with weight-based selection
5. device registry
6. active-device enforcement
7. free-cap enforcement
8. abuse scoring
9. rebalancer and lifecycle automation
10. dashboard and alerts

This sequence gives useful operational control early instead of waiting for the full system to be perfect.

---

## 21. Final Position

Escudo should operate like a modern traffic-engineered VPN, not like a collection of static server labels.

The system should be:

- class-based
- health-scored
- automatically allocated
- device-aware
- abuse-aware
- privacy-safe
- observable

That is the foundation required to launch, scale, and keep quality stable while the product grows.

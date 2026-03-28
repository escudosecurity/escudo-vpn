"""Digital Vulnerability Index calculation."""

from models import Municipality, VulnerabilityScore
from config import VULN_WEIGHTS, MOBILE_TECH_SCORES, SCORE_LABELS, SCORE_COLORS


def clamp(value: float, lo: float = 0.0, hi: float = 100.0) -> float:
    return max(lo, min(hi, value))


def calc_speed_score(muni: Municipality) -> float:
    """Slower download → higher score. 0 Mbps=100, 100+ Mbps=0."""
    dl = muni.speed.avg_download
    if dl <= 0:
        return 100.0
    if dl >= 100:
        return 0.0
    return clamp(100.0 - dl)


def calc_mobile_tech_score(muni: Municipality) -> float:
    """Weighted average of tech age. More 2G/3G → higher score."""
    tech_subs = muni.mobile.tech_subscribers
    total = sum(tech_subs.values())
    if total == 0:
        return 75.0  # no data = assume vulnerable

    score = 0.0
    for tech, subs in tech_subs.items():
        # Normalize tech name
        tech_upper = tech.upper().strip()
        if "5G" in tech_upper or "NR" in tech_upper:
            tech_key = "5G"
        elif "4G" in tech_upper or "LTE" in tech_upper:
            tech_key = "4G"
        elif "3G" in tech_upper or "UMTS" in tech_upper or "WCDMA" in tech_upper:
            tech_key = "3G"
        else:
            tech_key = "2G"
        weight = subs / total
        score += weight * MOBILE_TECH_SCORES.get(tech_key, 60)

    return clamp(score)


def calc_safety_score(muni: Municipality) -> float:
    """Use existing risk_score directly (0-100)."""
    return clamp(muni.safety.risk_score)


def calc_infrastructure_score(muni: Municipality) -> float:
    """School connectivity + sanitation coverage, inverted."""
    school_pct = muni.infrastructure.school_internet_pct  # 0-100
    water_pct = muni.infrastructure.water_coverage_pct  # 0-100
    sewage_pct = muni.infrastructure.sewage_coverage_pct  # 0-100

    # Average of all three, then invert (higher infra = lower vulnerability)
    infra_avg = (school_pct + water_pct + sewage_pct) / 3.0
    return clamp(100.0 - infra_avg)


def calc_provider_score(muni: Municipality) -> float:
    """Fewer mobile providers → higher score."""
    n = len(muni.mobile.providers)
    if n == 0:
        return 100.0
    if n >= 5:
        return 0.0
    # 1 provider=80, 2=60, 3=40, 4=20
    return clamp(100.0 - (n * 20.0))


def calc_tower_score(muni: Municipality, national_avg_towers_per_1k: float = 0.5) -> float:
    """Fewer towers per capita → higher score."""
    pop = muni.population
    towers = muni.towers.total_towers
    if pop == 0:
        return 75.0
    if towers == 0:
        return 90.0  # no tower data = very vulnerable

    towers_per_1k = (towers / pop) * 1000.0
    # Normalize: 0 towers/1k=100, national_avg*2=0
    threshold = national_avg_towers_per_1k * 2.0
    if threshold <= 0:
        threshold = 1.0
    score = 100.0 - (towers_per_1k / threshold) * 100.0
    return clamp(score)


def calculate_vulnerability(muni: Municipality, national_avg_towers_per_1k: float = 0.5) -> VulnerabilityScore:
    """Calculate the complete Digital Vulnerability Index for a municipality."""
    speed = calc_speed_score(muni)
    mobile = calc_mobile_tech_score(muni)
    safety = calc_safety_score(muni)
    infra = calc_infrastructure_score(muni)
    provider = calc_provider_score(muni)
    tower = calc_tower_score(muni, national_avg_towers_per_1k)

    total = (
        speed * VULN_WEIGHTS["speed"]
        + mobile * VULN_WEIGHTS["mobile_tech"]
        + safety * VULN_WEIGHTS["safety"]
        + infra * VULN_WEIGHTS["infrastructure"]
        + provider * VULN_WEIGHTS["provider_concentration"]
        + tower * VULN_WEIGHTS["tower_density"]
    )
    total = clamp(round(total, 1))

    # Determine label and color
    label = "Risco moderado"
    color = "yellow"
    for (lo, hi), (lbl, clr) in SCORE_LABELS.items():
        if lo <= total <= hi:
            label = lbl
            color = clr
            break

    return VulnerabilityScore(
        total=total,
        speed_score=round(speed, 1),
        mobile_tech_score=round(mobile, 1),
        safety_score=round(safety, 1),
        infrastructure_score=round(infra, 1),
        provider_score=round(provider, 1),
        tower_score=round(tower, 1),
        label=label,
        color=color,
        color_hex=SCORE_COLORS.get(color, "#d29922"),
    )

"""Data models for SEO page generation."""

from dataclasses import dataclass, field


@dataclass
class State:
    id: int
    name: str
    code: str
    abbrev: str


@dataclass
class SpeedData:
    avg_download: float = 0.0
    avg_upload: float = 0.0
    avg_latency: float = 0.0
    p10_download: float = 0.0
    p50_download: float = 0.0
    p90_download: float = 0.0
    p10_upload: float = 0.0
    p50_upload: float = 0.0
    p90_upload: float = 0.0


@dataclass
class MobileData:
    """Mobile subscriber data aggregated by technology."""
    tech_subscribers: dict = field(default_factory=dict)  # {"4G": 50000, "3G": 10000, ...}
    providers: list = field(default_factory=list)  # unique provider names
    total_subscribers: int = 0
    has_5g: bool = False


@dataclass
class SafetyData:
    homicide_rate: float = 0.0
    theft_rate: float = 0.0
    risk_score: float = 0.0


@dataclass
class InfrastructureData:
    school_internet_pct: float = 0.0
    total_schools: int = 0
    schools_with_internet: int = 0
    water_coverage_pct: float = 0.0
    sewage_coverage_pct: float = 0.0


@dataclass
class TowerData:
    total_towers: int = 0
    by_type: dict = field(default_factory=dict)  # {"LTE": 50, "GSM": 10, ...}


@dataclass
class BroadbandData:
    total_subscribers: int = 0
    providers: list = field(default_factory=list)
    by_technology: dict = field(default_factory=dict)


@dataclass
class VulnerabilityScore:
    total: float = 0.0
    speed_score: float = 0.0
    mobile_tech_score: float = 0.0
    safety_score: float = 0.0
    infrastructure_score: float = 0.0
    provider_score: float = 0.0
    tower_score: float = 0.0
    label: str = ""
    color: str = ""
    color_hex: str = ""


@dataclass
class Neighbor:
    id: int
    name: str
    code: str
    slug: str
    state_abbrev: str


@dataclass
class Municipality:
    id: int
    name: str
    code: str
    state_id: int
    state_abbrev: str
    state_name: str
    population: int = 0
    households: int = 0
    slug: str = ""
    speed: SpeedData = field(default_factory=SpeedData)
    mobile: MobileData = field(default_factory=MobileData)
    safety: SafetyData = field(default_factory=SafetyData)
    infrastructure: InfrastructureData = field(default_factory=InfrastructureData)
    towers: TowerData = field(default_factory=TowerData)
    broadband: BroadbandData = field(default_factory=BroadbandData)
    vulnerability: VulnerabilityScore = field(default_factory=VulnerabilityScore)
    neighbors: list = field(default_factory=list)  # List[Neighbor]
    broadband_penetration: float = 0.0

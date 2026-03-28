"""SQL queries and batch data loading from PostgreSQL."""

import psycopg2
import psycopg2.extras
from collections import defaultdict

from config import DB_CONFIG
from models import (
    State, Municipality, SpeedData, MobileData, SafetyData,
    InfrastructureData, TowerData, BroadbandData, Neighbor,
)
from slugify_br import slugify


def get_connection():
    return psycopg2.connect(**DB_CONFIG)


def load_states(conn) -> dict:
    """Load all Brazilian states. Returns {state_id: State}."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    cur.execute("""
        SELECT id, name, code, abbrev
        FROM admin_level_1
        WHERE country_code = 'BR'
        ORDER BY name
    """)
    states = {}
    for row in cur:
        states[row["id"]] = State(
            id=row["id"],
            name=row["name"],
            code=row["code"],
            abbrev=row["abbrev"].lower(),
        )
    cur.close()
    return states


def load_municipalities(conn, states: dict, state_filter: str = None) -> dict:
    """Load all BR municipalities. Returns {muni_id: Municipality}."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)

    sql = """
        SELECT id, name, code, l1_id, population, households
        FROM admin_level_2
        WHERE country_code = 'BR'
    """
    params = []
    if state_filter:
        state_ids = [s.id for s in states.values() if s.abbrev == state_filter.lower()]
        if state_ids:
            sql += " AND l1_id = %s"
            params.append(state_ids[0])

    sql += " ORDER BY name"
    cur.execute(sql, params)

    munis = {}
    for row in cur:
        state = states.get(row["l1_id"])
        if not state:
            continue
        m = Municipality(
            id=row["id"],
            name=row["name"],
            code=row["code"],
            state_id=row["l1_id"],
            state_abbrev=state.abbrev,
            state_name=state.name,
            population=row["population"] or 0,
            households=row["households"] or 0,
            slug=slugify(row["name"]),
        )
        munis[m.id] = m
    cur.close()
    return munis


def load_speed_data(conn, munis: dict):
    """Load speed test data — use latest quarter per municipality."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    cur.execute("""
        SELECT DISTINCT ON (l2_id)
               l2_id,
               avg_download_mbps, avg_upload_mbps, avg_latency_ms,
               p10_download_mbps, p50_download_mbps, p90_download_mbps
        FROM speedtest_municipality
        WHERE l2_id = ANY(%s)
        ORDER BY l2_id, quarter DESC
    """, ([m.id for m in munis.values()],))

    for row in cur:
        m = munis.get(row["l2_id"])
        if not m:
            continue
        dl = float(row["avg_download_mbps"] or 0)
        ul = float(row["avg_upload_mbps"] or 0)
        m.speed = SpeedData(
            avg_download=dl,
            avg_upload=ul,
            avg_latency=float(row["avg_latency_ms"] or 0),
            p10_download=float(row["p10_download_mbps"] or 0),
            p50_download=float(row["p50_download_mbps"] or 0),
            p90_download=float(row["p90_download_mbps"] or 0),
            # No upload percentiles in DB — estimate from ratio
            p10_upload=round(ul * 0.4, 2),
            p50_upload=round(ul * 0.85, 2),
            p90_upload=round(ul * 1.4, 2),
        )
    cur.close()


def load_mobile_data(conn, munis: dict):
    """Load mobile subscriber data — latest year_month per municipality."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    # Get latest data per municipality/provider/technology
    cur.execute("""
        SELECT DISTINCT ON (l2_id, provider_name, technology)
               l2_id, provider_name, technology, subscribers
        FROM mobile_subscribers
        WHERE l2_id = ANY(%s)
        ORDER BY l2_id, provider_name, technology, year_month DESC
    """, ([m.id for m in munis.values()],))

    tech_agg = defaultdict(lambda: defaultdict(int))
    providers_agg = defaultdict(set)

    for row in cur:
        mid = row["l2_id"]
        tech = row["technology"] or "Unknown"
        subs = int(row["subscribers"] or 0)
        tech_agg[mid][tech] += subs
        if row["provider_name"]:
            providers_agg[mid].add(row["provider_name"])

    for mid, m in munis.items():
        if mid in tech_agg:
            m.mobile = MobileData(
                tech_subscribers=dict(tech_agg[mid]),
                providers=sorted(providers_agg.get(mid, [])),
                total_subscribers=sum(tech_agg[mid].values()),
                has_5g=any("5G" in t.upper() or "NR" in t.upper() for t in tech_agg[mid]),
            )
    cur.close()


def load_safety_data(conn, munis: dict):
    """Load safety indicator data."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    cur.execute("""
        SELECT l2_id, homicide_rate, theft_rate, risk_score
        FROM safety_indicators
        WHERE l2_id = ANY(%s)
    """, ([m.id for m in munis.values()],))

    for row in cur:
        m = munis.get(row["l2_id"])
        if not m:
            continue
        m.safety = SafetyData(
            homicide_rate=float(row["homicide_rate"] or 0),
            theft_rate=float(row["theft_rate"] or 0),
            risk_score=float(row["risk_score"] or 0),
        )
    cur.close()


def load_infrastructure_data(conn, munis: dict):
    """Load school and sanitation data."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)

    # Schools
    cur.execute("""
        SELECT l2_id,
               COUNT(*) AS total_schools,
               SUM(CASE WHEN has_internet THEN 1 ELSE 0 END) AS schools_with_internet
        FROM schools
        WHERE l2_id = ANY(%s)
        GROUP BY l2_id
    """, ([m.id for m in munis.values()],))

    for row in cur:
        m = munis.get(row["l2_id"])
        if not m:
            continue
        total = int(row["total_schools"] or 0)
        with_internet = int(row["schools_with_internet"] or 0)
        m.infrastructure.total_schools = total
        m.infrastructure.schools_with_internet = with_internet
        m.infrastructure.school_internet_pct = (with_internet / total * 100.0) if total > 0 else 0.0

    # Sanitation
    cur.execute("""
        SELECT l2_id, water_coverage_pct, sewage_coverage_pct
        FROM sanitation_indicators
        WHERE l2_id = ANY(%s)
    """, ([m.id for m in munis.values()],))

    for row in cur:
        m = munis.get(row["l2_id"])
        if not m:
            continue
        m.infrastructure.water_coverage_pct = float(row["water_coverage_pct"] or 0)
        m.infrastructure.sewage_coverage_pct = float(row["sewage_coverage_pct"] or 0)

    cur.close()


def load_tower_data(conn, munis: dict):
    """Load OpenCelliD tower data."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    cur.execute("""
        SELECT l2_id, radio, COUNT(*) AS cnt
        FROM opencellid_towers
        WHERE l2_id = ANY(%s)
        GROUP BY l2_id, radio
    """, ([m.id for m in munis.values()],))

    tower_agg = defaultdict(lambda: defaultdict(int))
    for row in cur:
        tower_agg[row["l2_id"]][row["radio"]] = int(row["cnt"])

    for mid, m in munis.items():
        if mid in tower_agg:
            m.towers = TowerData(
                total_towers=sum(tower_agg[mid].values()),
                by_type=dict(tower_agg[mid]),
            )
    cur.close()


def load_broadband_data(conn, munis: dict):
    """Load broadband subscriber data."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    # Get latest data per municipality/technology
    cur.execute("""
        SELECT DISTINCT ON (l2_id, technology)
               l2_id, technology, provider_id, subscribers
        FROM broadband_subscribers
        WHERE l2_id = ANY(%s)
        ORDER BY l2_id, technology, year_month DESC
    """, ([m.id for m in munis.values()],))

    tech_agg = defaultdict(lambda: defaultdict(int))
    providers_agg = defaultdict(set)
    total_agg = defaultdict(int)

    for row in cur:
        mid = row["l2_id"]
        subs = int(row["subscribers"] or 0)
        tech_agg[mid][row["technology"] or "other"] += subs
        if row["provider_id"]:
            providers_agg[mid].add(str(row["provider_id"]))
        total_agg[mid] += subs

    for mid, m in munis.items():
        if mid in total_agg:
            m.broadband = BroadbandData(
                total_subscribers=total_agg[mid],
                providers=sorted(providers_agg.get(mid, [])),
                by_technology=dict(tech_agg[mid]),
            )
            if m.households > 0:
                m.broadband_penetration = round(total_agg[mid] / m.households * 100.0, 1)
    cur.close()


def load_neighbors(conn, munis: dict):
    """Load geographic neighbors using PostGIS ST_Touches."""
    cur = conn.cursor(cursor_factory=psycopg2.extras.DictCursor)
    muni_ids = list(munis.keys())
    cur.execute("""
        SELECT a.id AS src_id, b.id AS dst_id, b.name AS neighbor_name,
               b.code AS neighbor_code
        FROM admin_level_2 a
        JOIN admin_level_2 b ON ST_Touches(a.geom, b.geom)
        WHERE a.country_code = 'BR' AND b.country_code = 'BR'
          AND a.id = ANY(%s)
    """, (muni_ids,))

    for row in cur:
        src = munis.get(row["src_id"])
        dst = munis.get(row["dst_id"])
        if not src:
            continue
        if dst:
            neighbor = Neighbor(
                id=dst.id, name=dst.name, code=dst.code,
                slug=dst.slug, state_abbrev=dst.state_abbrev,
            )
        else:
            neighbor = Neighbor(
                id=row["dst_id"], name=row["neighbor_name"],
                code=row["neighbor_code"],
                slug=slugify(row["neighbor_name"]),
                state_abbrev="",
            )
        src.neighbors.append(neighbor)
    cur.close()


def load_all_data(state_filter: str = None) -> tuple:
    """Load all data from the database. Returns (states, municipalities)."""
    conn = get_connection()
    try:
        print("Loading states...")
        states = load_states(conn)
        print(f"  {len(states)} states loaded")

        print("Loading municipalities...")
        munis = load_municipalities(conn, states, state_filter)
        print(f"  {len(munis)} municipalities loaded")

        print("Loading speed data...")
        load_speed_data(conn, munis)

        print("Loading mobile data...")
        load_mobile_data(conn, munis)

        print("Loading safety data...")
        load_safety_data(conn, munis)

        print("Loading infrastructure data...")
        load_infrastructure_data(conn, munis)

        print("Loading tower data...")
        load_tower_data(conn, munis)

        print("Loading broadband data...")
        load_broadband_data(conn, munis)

        print("Loading neighbors (PostGIS)...")
        load_neighbors(conn, munis)

        return states, munis
    finally:
        conn.close()

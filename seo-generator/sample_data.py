"""Generate realistic sample data for testing without DB access."""

import random
import math
from models import (
    State, Municipality, SpeedData, MobileData, SafetyData,
    InfrastructureData, TowerData, BroadbandData, Neighbor,
)
from slugify_br import slugify

# All 27 Brazilian states
STATES_DATA = [
    (1, "Acre", "BR-AC", "ac"),
    (2, "Alagoas", "BR-AL", "al"),
    (3, "Amapa", "BR-AP", "ap"),
    (4, "Amazonas", "BR-AM", "am"),
    (5, "Bahia", "BR-BA", "ba"),
    (6, "Ceara", "BR-CE", "ce"),
    (7, "Distrito Federal", "BR-DF", "df"),
    (8, "Espirito Santo", "BR-ES", "es"),
    (9, "Goias", "BR-GO", "go"),
    (10, "Maranhao", "BR-MA", "ma"),
    (11, "Mato Grosso", "BR-MT", "mt"),
    (12, "Mato Grosso do Sul", "BR-MS", "ms"),
    (13, "Minas Gerais", "BR-MG", "mg"),
    (14, "Para", "BR-PA", "pa"),
    (15, "Paraiba", "BR-PB", "pb"),
    (16, "Parana", "BR-PR", "pr"),
    (17, "Pernambuco", "BR-PE", "pe"),
    (18, "Piaui", "BR-PI", "pi"),
    (19, "Rio de Janeiro", "BR-RJ", "rj"),
    (20, "Rio Grande do Norte", "BR-RN", "rn"),
    (21, "Rio Grande do Sul", "BR-RS", "rs"),
    (22, "Rondonia", "BR-RO", "ro"),
    (23, "Roraima", "BR-RR", "rr"),
    (24, "Santa Catarina", "BR-SC", "sc"),
    (25, "Sao Paulo", "BR-SP", "sp"),
    (26, "Sergipe", "BR-SE", "se"),
    (27, "Tocantins", "BR-TO", "to"),
]

# Sample municipalities per state (subset for testing)
PR_MUNICIPALITIES = [
    "Curitiba", "Londrina", "Maringa", "Ponta Grossa", "Cascavel",
    "Sao Jose dos Pinhais", "Foz do Iguacu", "Colombo", "Guarapuava",
    "Paranagua", "Araucaria", "Toledo", "Apucarana", "Pinhais",
    "Campo Largo", "Arapongas", "Almirante Tamandare", "Umuarama",
    "Piraquara", "Cambe", "Campo Mourao", "Fazenda Rio Grande",
    "Francisco Beltrao", "Pato Branco", "Cianorte", "Telêmaco Borba",
    "Castro", "Rolandia", "Irati", "Uniao da Vitoria",
    "Palmas", "Sarandi", "Ibipora", "Lapa", "Cornelio Procopio",
    "Prudentopolis", "Rio Negro", "Ivaipora", "Wenceslau Braz",
    "Matinhos",
]

SP_MUNICIPALITIES = [
    "Sao Paulo", "Guarulhos", "Campinas", "Sao Bernardo do Campo",
    "Santo Andre", "Osasco", "Sao Jose dos Campos", "Ribeirao Preto",
    "Sorocaba", "Santos", "Maua", "Sao Jose do Rio Preto",
    "Mogi das Cruzes", "Diadema", "Jundiai", "Piracicaba",
    "Carapicuiba", "Bauru", "Itaquaquecetuba", "Sao Vicente",
]

SMALL_STATE_MUNIS = {
    "ac": ["Rio Branco", "Cruzeiro do Sul", "Sena Madureira", "Tarauaca", "Feijo"],
    "al": ["Maceio", "Arapiraca", "Rio Largo", "Palmeira dos Indios", "Penedo"],
    "ap": ["Macapa", "Santana", "Laranjal do Jari", "Oiapoque", "Mazagao"],
    "am": ["Manaus", "Parintins", "Itacoatiara", "Manacapuru", "Coari"],
    "ba": ["Salvador", "Feira de Santana", "Vitoria da Conquista", "Camacari", "Ilheus", "Itabuna", "Juazeiro", "Lauro de Freitas"],
    "ce": ["Fortaleza", "Caucaia", "Juazeiro do Norte", "Maracanau", "Sobral", "Crato"],
    "df": ["Brasilia"],
    "es": ["Vitoria", "Vila Velha", "Serra", "Cariacica", "Cachoeiro de Itapemirim"],
    "go": ["Goiania", "Aparecida de Goiania", "Anapolis", "Rio Verde", "Luziania"],
    "ma": ["Sao Luis", "Imperatriz", "Caxias", "Timon", "Codó"],
    "mt": ["Cuiaba", "Varzea Grande", "Rondonopolis", "Sinop", "Tangara da Serra"],
    "ms": ["Campo Grande", "Dourados", "Três Lagoas", "Corumba", "Ponta Pora"],
    "mg": ["Belo Horizonte", "Uberlandia", "Contagem", "Juiz de Fora", "Betim", "Montes Claros", "Uberaba", "Governador Valadares"],
    "pa": ["Belem", "Ananindeua", "Santarem", "Maraba", "Castanhal"],
    "pb": ["Joao Pessoa", "Campina Grande", "Santa Rita", "Patos", "Bayeux"],
    "pe": ["Recife", "Jaboatao dos Guararapes", "Olinda", "Caruaru", "Petrolina", "Paulista"],
    "pi": ["Teresina", "Parnaiba", "Picos", "Piripiri", "Floriano"],
    "rj": ["Rio de Janeiro", "Sao Goncalo", "Duque de Caxias", "Nova Iguacu", "Niteroi", "Campos dos Goytacazes", "Belford Roxo", "Petropolis"],
    "rn": ["Natal", "Mossoro", "Parnamirim", "Sao Goncalo do Amarante", "Macaiba"],
    "rs": ["Porto Alegre", "Caxias do Sul", "Pelotas", "Canoas", "Santa Maria", "Gravatai", "Viamao", "Novo Hamburgo"],
    "ro": ["Porto Velho", "Ji-Parana", "Ariquemes", "Vilhena", "Cacoal"],
    "rr": ["Boa Vista", "Rorainopolis", "Caracarai", "Alto Alegre", "Pacaraima"],
    "sc": ["Florianopolis", "Joinville", "Blumenau", "Sao Jose", "Chapeco", "Criciuma", "Itajai"],
    "se": ["Aracaju", "Nossa Senhora do Socorro", "Lagarto", "Itabaiana", "Estancia"],
    "to": ["Palmas", "Araguaina", "Gurupi", "Porto Nacional", "Paraiso do Tocantins"],
}


def _random_speed(is_capital: bool) -> SpeedData:
    """Generate realistic speed data based on city type."""
    if is_capital:
        base_dl = random.uniform(40, 120)
    else:
        base_dl = random.uniform(8, 80)

    base_ul = base_dl * random.uniform(0.3, 0.6)

    return SpeedData(
        avg_download=round(base_dl, 2),
        avg_upload=round(base_ul, 2),
        avg_latency=round(random.uniform(8, 60), 2),
        p10_download=round(base_dl * 0.3, 2),
        p50_download=round(base_dl * 0.8, 2),
        p90_download=round(base_dl * 1.5, 2),
        p10_upload=round(base_ul * 0.3, 2),
        p50_upload=round(base_ul * 0.8, 2),
        p90_upload=round(base_ul * 1.5, 2),
    )


def _random_mobile(pop: int, is_capital: bool) -> MobileData:
    providers = ["Claro", "Vivo", "TIM", "Oi"]
    if is_capital:
        active_providers = providers[:]
        if random.random() > 0.5:
            active_providers.append("Algar")
    else:
        n = random.randint(2, 4)
        active_providers = random.sample(providers, n)

    total = int(pop * random.uniform(0.6, 1.2))
    techs = {}
    if is_capital and random.random() > 0.4:
        techs["5G"] = int(total * random.uniform(0.05, 0.15))
    techs["4G"] = int(total * random.uniform(0.4, 0.65))
    techs["3G"] = int(total * random.uniform(0.15, 0.3))
    techs["2G"] = int(total * random.uniform(0.02, 0.1))

    return MobileData(
        tech_subscribers=techs,
        providers=sorted(active_providers),
        total_subscribers=sum(techs.values()),
        has_5g="5G" in techs,
    )


def _random_safety() -> SafetyData:
    risk = random.uniform(10, 85)
    return SafetyData(
        homicide_rate=round(random.uniform(2, 50), 1),
        theft_rate=round(random.uniform(50, 800), 1),
        risk_score=round(risk, 1),
    )


def _random_infra(is_capital: bool) -> InfrastructureData:
    if is_capital:
        school_pct = random.uniform(70, 98)
        water = random.uniform(80, 99)
        sewage = random.uniform(60, 95)
    else:
        school_pct = random.uniform(30, 90)
        water = random.uniform(40, 95)
        sewage = random.uniform(10, 80)

    total_schools = random.randint(5, 200)
    with_internet = int(total_schools * school_pct / 100)

    return InfrastructureData(
        school_internet_pct=round(school_pct, 1),
        total_schools=total_schools,
        schools_with_internet=with_internet,
        water_coverage_pct=round(water, 1),
        sewage_coverage_pct=round(sewage, 1),
    )


def _random_towers(pop: int) -> TowerData:
    density = random.uniform(0.1, 1.5)  # towers per 1k people
    total = max(1, int(pop / 1000 * density))
    by_type = {}
    remaining = total
    for radio in ["LTE", "UMTS", "GSM"]:
        pct = random.uniform(0.1, 0.5) if remaining > 0 else 0
        count = max(0, int(total * pct))
        count = min(count, remaining)
        by_type[radio] = count
        remaining -= count
    if remaining > 0:
        by_type["NR"] = remaining

    return TowerData(total_towers=total, by_type=by_type)


def _random_broadband(pop: int, households: int) -> BroadbandData:
    penetration = random.uniform(0.3, 0.9)
    total_subs = int(households * penetration)
    providers = random.sample(["Claro", "Vivo", "Oi", "TIM", "Brisanet", "Desktop", "Algar"], random.randint(2, 5))
    techs = {
        "Fibra": int(total_subs * random.uniform(0.3, 0.7)),
        "Cable": int(total_subs * random.uniform(0.1, 0.3)),
        "DSL": int(total_subs * random.uniform(0.05, 0.2)),
        "Radio": int(total_subs * random.uniform(0.01, 0.1)),
    }
    return BroadbandData(
        total_subscribers=total_subs,
        providers=sorted(providers),
        by_technology=techs,
    )


def generate_sample_data(state_filter: str = None) -> tuple:
    """Generate sample data for all states/municipalities."""
    random.seed(42)  # reproducible

    states = {}
    for sid, name, code, abbrev in STATES_DATA:
        states[sid] = State(id=sid, name=name, code=code, abbrev=abbrev)

    munis = {}
    muni_id = 1000

    state_munis_map = dict(SMALL_STATE_MUNIS)
    state_munis_map["pr"] = PR_MUNICIPALITIES
    state_munis_map["sp"] = SP_MUNICIPALITIES

    for state in states.values():
        if state_filter and state.abbrev != state_filter.lower():
            continue

        city_names = state_munis_map.get(state.abbrev, [f"Cidade {i}" for i in range(1, 6)])

        for i, city_name in enumerate(city_names):
            muni_id += 1
            is_capital = (i == 0)
            pop = random.randint(200000, 12000000) if is_capital else random.randint(5000, 500000)
            households = int(pop / random.uniform(2.8, 3.5))

            m = Municipality(
                id=muni_id,
                name=city_name,
                code=f"BR-{state.abbrev.upper()}-{muni_id}",
                state_id=state.id,
                state_abbrev=state.abbrev,
                state_name=state.name,
                population=pop,
                households=households,
                slug=slugify(city_name),
                speed=_random_speed(is_capital),
                mobile=_random_mobile(pop, is_capital),
                safety=_random_safety(),
                infrastructure=_random_infra(is_capital),
                towers=_random_towers(pop),
                broadband=_random_broadband(pop, households),
            )
            if m.households > 0:
                m.broadband_penetration = round(m.broadband.total_subscribers / m.households * 100, 1)
            munis[m.id] = m

    # Generate neighbor relationships (link sequential cities in same state)
    by_state = {}
    for m in munis.values():
        by_state.setdefault(m.state_abbrev, []).append(m)

    for state_abbrev, state_munis in by_state.items():
        for i, m in enumerate(state_munis):
            neighbors = []
            if i > 0:
                prev = state_munis[i - 1]
                neighbors.append(Neighbor(
                    id=prev.id, name=prev.name, code=prev.code,
                    slug=prev.slug, state_abbrev=prev.state_abbrev,
                ))
            if i < len(state_munis) - 1:
                nxt = state_munis[i + 1]
                neighbors.append(Neighbor(
                    id=nxt.id, name=nxt.name, code=nxt.code,
                    slug=nxt.slug, state_abbrev=nxt.state_abbrev,
                ))
            # Add a random neighbor from same state
            others = [x for x in state_munis if x.id != m.id and x.id not in [n.id for n in neighbors]]
            if others:
                r = random.choice(others)
                neighbors.append(Neighbor(
                    id=r.id, name=r.name, code=r.code,
                    slug=r.slug, state_abbrev=r.state_abbrev,
                ))
            m.neighbors = neighbors

    print(f"Generated sample data: {len(states)} states, {len(munis)} municipalities")
    return states, munis

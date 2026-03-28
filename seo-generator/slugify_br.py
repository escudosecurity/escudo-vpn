"""Brazilian municipality name slugification."""

import re
from unidecode import unidecode


def slugify(name: str) -> str:
    """Convert a Brazilian municipality name to a URL slug.

    Examples:
        São Paulo → sao-paulo
        Foz do Iguaçu → foz-do-iguacu
        Pôrto Velho → porto-velho
        São José dos Campos → sao-jose-dos-campos
    """
    s = unidecode(name)
    s = s.lower()
    s = re.sub(r"[^a-z0-9\s-]", "", s)
    s = re.sub(r"[\s]+", "-", s.strip())
    s = re.sub(r"-+", "-", s)
    return s

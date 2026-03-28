package com.escudo.vpn.ui.util

import java.util.Locale

fun countryCodeToFlag(countryCode: String): String {
    return countryCode.uppercase()
        .map { char -> String(Character.toChars(0x1F1E6 - 'A'.code + char.code)) }
        .joinToString("")
}

fun countryCodeToDisplayName(countryCode: String?): String? {
    val normalized = countryCode
        ?.trim()
        ?.uppercase()
        ?.takeIf { it.length == 2 }
        ?: return null

    val locale = Locale("", normalized)
    return locale.getDisplayCountry(Locale("pt", "BR"))
        .takeIf { it.isNotBlank() && !it.equals(normalized, ignoreCase = true) }
}

fun formatSpeed(bytes: Long): String {
    val mbps = bytes * 8.0 / 1_000_000
    return if (mbps >= 1.0) "%.1f Mbps".format(mbps)
    else "%.0f Kbps".format(mbps * 1000)
}

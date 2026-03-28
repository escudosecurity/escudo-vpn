package com.escudo.vpn.ui.model

import com.escudo.vpn.data.model.Server

enum class ServerCategory(
    val sectionTitle: String,
    val badgeLabel: String,
    val exitLabel: String
) {
    RESIDENTIAL_US(
        sectionTitle = "Residential Routes",
        badgeLabel = "Residential US",
        exitLabel = "Shared residential exit in the United States"
    ),
    RESIDENTIAL_UK(
        sectionTitle = "Residential Routes",
        badgeLabel = "Residential UK",
        exitLabel = "Shared residential exit in the United Kingdom"
    ),
    RESIDENTIAL_EU(
        sectionTitle = "Residential Routes",
        badgeLabel = "Residential EU",
        exitLabel = "Shared residential exit in Europe"
    ),
    STANDARD(
        sectionTitle = "Standard Servers",
        badgeLabel = "Standard",
        exitLabel = "Direct secure route"
    )
}

data class ServerPresentation(
    val title: String,
    val subtitle: String,
    val category: ServerCategory,
    val serviceClassLabel: String
)

fun Server.toPresentation(): ServerPresentation {
    val normalizedCountry = countryCode?.trim()?.uppercase().orEmpty()
    val normalizedServiceClass = serviceClass?.trim().orEmpty()
    val isResidentialShared = normalizedServiceClass.equals("Medium", ignoreCase = true)

    val category = when {
        isResidentialShared && normalizedCountry == "US" -> ServerCategory.RESIDENTIAL_US
        isResidentialShared && normalizedCountry == "GB" -> ServerCategory.RESIDENTIAL_UK
        isResidentialShared && (normalizedCountry == "DE" || normalizedCountry == "NL") -> {
            ServerCategory.RESIDENTIAL_EU
        }
        else -> ServerCategory.STANDARD
    }

    val title = when (category) {
        ServerCategory.RESIDENTIAL_US -> "Residential US"
        ServerCategory.RESIDENTIAL_UK -> "Residential UK"
        ServerCategory.RESIDENTIAL_EU -> "Residential EU"
        ServerCategory.STANDARD -> when {
            location.contains("Singapore", ignoreCase = true) -> "Singapore"
            location.contains("Sydney", ignoreCase = true) -> "Sydney"
            location.contains("Bangalore", ignoreCase = true) -> "Bangalore"
            location.contains("Toronto", ignoreCase = true) -> "Toronto"
            location.contains("London", ignoreCase = true) -> "London"
            location.contains("Ashburn", ignoreCase = true) -> "Ashburn"
            location.contains("Hillsboro", ignoreCase = true) -> "Hillsboro"
            location.contains("Helsinki", ignoreCase = true) -> "Helsinki"
            location.contains("Falkenstein", ignoreCase = true) -> "Falkenstein"
            location.contains("Nuremberg", ignoreCase = true) -> "Nuremberg"
            location.contains("Amsterdam", ignoreCase = true) -> "Amsterdam"
            location.contains("New Jersey", ignoreCase = true) -> "New Jersey"
            location.contains("São Paulo", ignoreCase = true) -> "São Paulo"
            else -> location.substringBefore(",").ifBlank { name }
        }
    }

    val subtitle = if (category == ServerCategory.STANDARD) {
        location
    } else {
        category.exitLabel
    }

    return ServerPresentation(
        title = title,
        subtitle = subtitle,
        category = category,
        serviceClassLabel = normalizedServiceClass.ifBlank { "Free" }
    )
}

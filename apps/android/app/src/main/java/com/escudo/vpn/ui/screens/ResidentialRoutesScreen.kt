package com.escudo.vpn.ui.screens

import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Public
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.hilt.navigation.compose.hiltViewModel
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.components.ServerCard
import com.escudo.vpn.ui.model.ServerCategory
import com.escudo.vpn.ui.model.toPresentation
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.TextPrimary
import com.escudo.vpn.ui.theme.TextSecondary

@Composable
fun ResidentialRoutesScreen(
    onRouteSelected: () -> Unit,
    viewModel: ServersViewModel = hiltViewModel()
) {
    val servers by viewModel.servers.collectAsState()
    val selectedServerId by viewModel.selectedServerId.collectAsState()

    val residentialRoutes = listOf(
        ServerCategory.RESIDENTIAL_US,
        ServerCategory.RESIDENTIAL_UK,
        ServerCategory.RESIDENTIAL_EU
    ).mapNotNull { category ->
        servers.firstOrNull { it.toPresentation().category == category }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
            .padding(20.dp)
    ) {
        Text(
            text = "Residential Routes",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Premium shared residential exits. One clean route per region.",
            style = MaterialTheme.typography.bodyMedium,
            color = TextSecondary
        )

        Spacer(modifier = Modifier.height(16.dp))

        Box(
            modifier = Modifier
                .fillMaxWidth()
                .background(CardBackground, RoundedCornerShape(22.dp))
                .border(1.dp, Accent.copy(alpha = 0.12f), RoundedCornerShape(22.dp))
                .padding(18.dp)
        ) {
            Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
                Text(
                    text = "Power and Family plans use these routes for streaming and higher-quality exits.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextPrimary
                )
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    FleetFact("US", if (residentialRoutes.any { it.toPresentation().category == ServerCategory.RESIDENTIAL_US }) "Live" else "--")
                    FleetFact("UK", if (residentialRoutes.any { it.toPresentation().category == ServerCategory.RESIDENTIAL_UK }) "Live" else "--")
                    FleetFact("EU", if (residentialRoutes.any { it.toPresentation().category == ServerCategory.RESIDENTIAL_EU }) "Live" else "--")
                }
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(10.dp)
                ) {
                    ResidentialHint(Icons.Default.Public, "Shared residential")
                    ResidentialHint(Icons.Default.Bolt, "Streaming-ready")
                    ResidentialHint(Icons.Default.Lock, "Private exit")
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        LazyColumn(
            contentPadding = PaddingValues(vertical = 8.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp)
        ) {
            items(residentialRoutes, key = { it.id }) { server ->
                ServerCard(
                    server = server,
                    isSelected = server.id == selectedServerId,
                    onClick = {
                        viewModel.selectServer(server)
                        onRouteSelected()
                    }
                )
            }
        }
    }
}

@Composable
private fun FleetFact(
    label: String,
    value: String
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleLarge,
            color = Accent
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = TextSecondary
        )
    }
}

@Composable
private fun ResidentialHint(
    icon: androidx.compose.ui.graphics.vector.ImageVector,
    text: String
) {
    Row(
        modifier = Modifier
            .background(Background, RoundedCornerShape(999.dp))
            .padding(horizontal = 12.dp, vertical = 8.dp),
        horizontalArrangement = Arrangement.spacedBy(6.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = Accent
        )
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium,
            color = TextSecondary
        )
    }
}

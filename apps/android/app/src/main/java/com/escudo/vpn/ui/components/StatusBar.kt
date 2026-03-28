package com.escudo.vpn.ui.components

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.ArrowDownward
import androidx.compose.material.icons.filled.ArrowUpward
import androidx.compose.material.icons.filled.Timer
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.unit.dp
import com.escudo.vpn.data.model.TrafficStats
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.ConnectedGreen
import com.escudo.vpn.ui.theme.TextSecondary

@Composable
fun StatusBar(
    trafficStats: TrafficStats,
    connectionTimeSecs: Long,
    modifier: Modifier = Modifier
) {
    Row(
        modifier = modifier
            .fillMaxWidth()
            .clip(RoundedCornerShape(12.dp))
            .background(CardBackground)
            .padding(16.dp),
        horizontalArrangement = Arrangement.SpaceEvenly
    ) {
        StatItem(
            icon = {
                Icon(
                    imageVector = Icons.Default.ArrowDownward,
                    contentDescription = "Recebidos",
                    tint = ConnectedGreen
                )
            },
            label = "Download",
            value = formatBytes(trafficStats.rxBytes)
        )

        StatItem(
            icon = {
                Icon(
                    imageVector = Icons.Default.ArrowUpward,
                    contentDescription = "Enviados",
                    tint = Accent
                )
            },
            label = "Upload",
            value = formatBytes(trafficStats.txBytes)
        )

        StatItem(
            icon = {
                Icon(
                    imageVector = Icons.Default.Timer,
                    contentDescription = "Tempo de conexão",
                    tint = TextSecondary
                )
            },
            label = "Tempo",
            value = formatDuration(connectionTimeSecs)
        )
    }
}

@Composable
private fun StatItem(
    icon: @Composable () -> Unit,
    label: String,
    value: String
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally,
        verticalArrangement = Arrangement.spacedBy(4.dp)
    ) {
        icon()
        Text(
            text = value,
            style = MaterialTheme.typography.titleMedium
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = TextSecondary
        )
    }
}

private fun formatBytes(bytes: Long): String {
    return when {
        bytes < 1024 -> "$bytes B"
        bytes < 1024 * 1024 -> "%.1f KB".format(bytes / 1024.0)
        bytes < 1024 * 1024 * 1024 -> "%.1f MB".format(bytes / (1024.0 * 1024.0))
        else -> "%.2f GB".format(bytes / (1024.0 * 1024.0 * 1024.0))
    }
}

private fun formatDuration(totalSeconds: Long): String {
    val hours = totalSeconds / 3600
    val minutes = (totalSeconds % 3600) / 60
    val seconds = totalSeconds % 60
    return if (hours > 0) {
        "%02d:%02d:%02d".format(hours, minutes, seconds)
    } else {
        "%02d:%02d".format(minutes, seconds)
    }
}

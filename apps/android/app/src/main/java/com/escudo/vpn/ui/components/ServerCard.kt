package com.escudo.vpn.ui.components

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.CheckCircle
import androidx.compose.material.icons.filled.Public
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.escudo.vpn.data.model.Server
import com.escudo.vpn.ui.model.ServerCategory
import com.escudo.vpn.ui.model.toPresentation
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.BadgeDirect
import com.escudo.vpn.ui.theme.BadgePower
import com.escudo.vpn.ui.theme.BadgeResidential
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.ConnectedGreen
import com.escudo.vpn.ui.theme.ErrorRed
import com.escudo.vpn.ui.theme.TextPrimary
import com.escudo.vpn.ui.theme.TextSecondary

@Composable
fun ServerCard(
    server: Server,
    isSelected: Boolean,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val presentation = server.toPresentation()
    val loadColor = when {
        server.loadPercent < 50 -> ConnectedGreen
        server.loadPercent < 80 -> Accent
        else -> ErrorRed
    }

    Card(
        modifier = modifier
            .fillMaxWidth()
            .clickable(onClick = onClick),
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = CardBackground),
        border = if (isSelected) {
            BorderStroke(1.dp, Accent)
        } else {
            BorderStroke(1.dp, Color.White.copy(alpha = 0.05f))
        }
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(18.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .size(44.dp)
                    .clip(CircleShape)
                    .background(Accent.copy(alpha = 0.12f))
                    .border(1.dp, Accent.copy(alpha = 0.18f), CircleShape),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Public,
                    contentDescription = null,
                    tint = Accent,
                    modifier = Modifier.size(24.dp)
                )
            }

            Spacer(modifier = Modifier.width(12.dp))

            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = presentation.title,
                    style = MaterialTheme.typography.titleMedium
                )
                Spacer(modifier = Modifier.height(2.dp))
                Text(
                    text = presentation.subtitle,
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary
                )
                Spacer(modifier = Modifier.height(8.dp))
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    ServerBadge(
                        text = presentation.category.badgeLabel,
                        category = presentation.category
                    )
                    ServiceClassBadge(text = presentation.serviceClassLabel)
                }
                Spacer(modifier = Modifier.height(12.dp))
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    LinearProgressIndicator(
                        progress = server.loadPercent / 100f,
                        modifier = Modifier
                            .weight(1f)
                            .height(4.dp)
                            .clip(RoundedCornerShape(2.dp)),
                        color = loadColor,
                        trackColor = loadColor.copy(alpha = 0.15f)
                    )
                    Text(
                        text = "${server.loadPercent.toInt()}%",
                        style = MaterialTheme.typography.labelMedium,
                        color = loadColor
                    )
                }
            }

            if (isSelected) {
                Spacer(modifier = Modifier.width(8.dp))
                Icon(
                    imageVector = Icons.Default.CheckCircle,
                    contentDescription = "Selecionado",
                    tint = Accent,
                    modifier = Modifier.size(24.dp)
                )
            }
        }
    }
}

@Composable
private fun ServerBadge(
    text: String,
    category: ServerCategory
) {
    val background = when (category) {
        ServerCategory.STANDARD -> BadgeDirect
        ServerCategory.RESIDENTIAL_EU,
        ServerCategory.RESIDENTIAL_UK,
        ServerCategory.RESIDENTIAL_US -> BadgeResidential
    }

    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(999.dp))
            .background(background)
            .border(1.dp, Accent.copy(alpha = 0.2f), RoundedCornerShape(999.dp))
            .padding(horizontal = 10.dp, vertical = 5.dp)
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium,
            color = when (category) {
                ServerCategory.STANDARD -> TextSecondary
                ServerCategory.RESIDENTIAL_EU,
                ServerCategory.RESIDENTIAL_UK,
                ServerCategory.RESIDENTIAL_US -> Accent
            }
        )
    }
}

@Composable
private fun ServiceClassBadge(text: String) {
    val isPower = text.equals("Power", ignoreCase = true)
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(999.dp))
            .background(if (isPower) BadgePower else BadgeDirect)
            .border(
                1.dp,
                if (isPower) BadgePower else Accent.copy(alpha = 0.2f),
                RoundedCornerShape(999.dp)
            )
            .padding(horizontal = 10.dp, vertical = 5.dp)
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium,
            color = if (isPower) Color.White else TextPrimary
        )
    }
}

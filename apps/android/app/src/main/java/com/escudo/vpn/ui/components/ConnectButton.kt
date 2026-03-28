package com.escudo.vpn.ui.components

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.PowerSettingsNew
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.escudo.vpn.data.model.ConnectionState
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.ConnectedGreen
import com.escudo.vpn.ui.theme.DisabledGray
import com.escudo.vpn.ui.theme.Surface

@Composable
fun ConnectButton(
    connectionState: ConnectionState,
    onClick: () -> Unit,
    modifier: Modifier = Modifier
) {
    val isConnecting = connectionState == ConnectionState.CONNECTING ||
            connectionState == ConnectionState.DISCONNECTING

    val buttonColor by animateColorAsState(
        targetValue = when (connectionState) {
            ConnectionState.CONNECTED -> ConnectedGreen
            ConnectionState.CONNECTING -> Accent.copy(alpha = 0.6f)
            ConnectionState.DISCONNECTING -> DisabledGray
            ConnectionState.DISCONNECTED -> Accent
        },
        animationSpec = tween(durationMillis = 500),
        label = "buttonColor"
    )

    val borderColor by animateColorAsState(
        targetValue = when (connectionState) {
            ConnectionState.CONNECTED -> ConnectedGreen.copy(alpha = 0.4f)
            ConnectionState.CONNECTING -> Accent.copy(alpha = 0.3f)
            ConnectionState.DISCONNECTING -> DisabledGray.copy(alpha = 0.3f)
            ConnectionState.DISCONNECTED -> Accent.copy(alpha = 0.3f)
        },
        animationSpec = tween(durationMillis = 500),
        label = "borderColor"
    )

    val scale by animateFloatAsState(
        targetValue = if (isConnecting) 0.95f else 1f,
        animationSpec = tween(durationMillis = 300),
        label = "scale"
    )

    Box(
        contentAlignment = Alignment.Center,
        modifier = modifier
    ) {
        // Outer glow ring
        Box(
            modifier = Modifier
                .size(180.dp)
                .scale(scale)
                .clip(CircleShape)
                .border(
                    width = 2.dp,
                    brush = Brush.radialGradient(
                        colors = listOf(borderColor, Color.Transparent)
                    ),
                    shape = CircleShape
                )
                .background(
                    brush = Brush.radialGradient(
                        colors = listOf(
                            buttonColor.copy(alpha = 0.1f),
                            Color.Transparent
                        )
                    )
                ),
            contentAlignment = Alignment.Center
        ) {
            // Inner button
            Box(
                modifier = Modifier
                    .size(140.dp)
                    .clip(CircleShape)
                    .background(
                        brush = Brush.radialGradient(
                            colors = listOf(
                                buttonColor.copy(alpha = 0.2f),
                                Surface
                            )
                        )
                    )
                    .border(
                        width = 3.dp,
                        color = buttonColor,
                        shape = CircleShape
                    )
                    .clickable(
                        interactionSource = remember { MutableInteractionSource() },
                        indication = null,
                        enabled = !isConnecting,
                        onClick = onClick
                    ),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.PowerSettingsNew,
                    contentDescription = "Conectar VPN",
                    tint = buttonColor,
                    modifier = Modifier.size(56.dp)
                )
            }
        }
    }
}

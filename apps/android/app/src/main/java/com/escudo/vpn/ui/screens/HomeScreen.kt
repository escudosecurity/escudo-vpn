package com.escudo.vpn.ui.screens

import android.app.Activity
import android.content.ComponentName
import android.content.Context
import android.content.Intent
import android.content.ServiceConnection
import android.net.VpnService
import android.os.IBinder
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.Image
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Bolt
import androidx.compose.material.icons.filled.KeyboardArrowRight
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Public
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.DisposableEffect
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.alpha
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.res.painterResource
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.em
import androidx.compose.ui.unit.sp
import androidx.core.content.ContextCompat
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.escudo.vpn.R
import com.escudo.vpn.data.model.ConnectionState
import com.escudo.vpn.data.model.DnsStats
import com.escudo.vpn.data.model.NetworkInfo
import com.escudo.vpn.data.model.Server
import com.escudo.vpn.data.model.TrafficStats
import com.escudo.vpn.data.repository.VpnRepository
import com.escudo.vpn.service.EscudoVpnService
import com.escudo.vpn.service.TunnelManager
import com.escudo.vpn.ui.components.ConnectButton
import com.escudo.vpn.ui.components.EscudoCard
import com.escudo.vpn.ui.components.StatusBar
import com.escudo.vpn.ui.model.toPresentation
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.AccentDark
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.ConnectedGreen
import com.escudo.vpn.ui.theme.MonoLabel
import com.escudo.vpn.ui.theme.TextPrimary
import com.escudo.vpn.ui.theme.TextSecondary
import com.escudo.vpn.ui.util.formatSpeed
import dagger.hilt.android.lifecycle.HiltViewModel
import kotlin.math.min
import javax.inject.Inject
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@HiltViewModel
class HomeViewModel @Inject constructor(
    private val vpnRepository: VpnRepository,
    private val tunnelManager: TunnelManager
) : ViewModel() {

    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _trafficStats = MutableStateFlow(TrafficStats())
    val trafficStats: StateFlow<TrafficStats> = _trafficStats.asStateFlow()

    private val _connectionTime = MutableStateFlow(0L)
    val connectionTime: StateFlow<Long> = _connectionTime.asStateFlow()

    private val _dnsStats = MutableStateFlow(DnsStats())
    val dnsStats: StateFlow<DnsStats> = _dnsStats.asStateFlow()

    private val _isPrivateMode = MutableStateFlow(false)
    val isPrivateMode: StateFlow<Boolean> = _isPrivateMode.asStateFlow()

    private val _error = MutableStateFlow<String?>(null)
    val error: StateFlow<String?> = _error.asStateFlow()

    private val _networkInfo = MutableStateFlow<NetworkInfo?>(null)
    val networkInfo: StateFlow<NetworkInfo?> = _networkInfo.asStateFlow()

    private var vpnService: EscudoVpnService? = null

    private val _selectedServerName =
        MutableStateFlow(vpnRepository.getSelectedServerName() ?: "Servidor automático")
    val selectedServerName: StateFlow<String> = _selectedServerName.asStateFlow()

    private val _selectedServer = MutableStateFlow<Server?>(null)
    val selectedServer: StateFlow<Server?> = _selectedServer.asStateFlow()

    val selectedServerId: String?
        get() = vpnRepository.getSelectedServerId()

    init {
        autoSelectServerIfNeeded()
        refreshSelectedServer()
        refreshNetworkInfo()
        startDnsStatsPolling()
    }

    private fun startDnsStatsPolling() {
        viewModelScope.launch {
            _connectionState.collect { state ->
                if (state == ConnectionState.CONNECTED) {
                    while (_connectionState.value == ConnectionState.CONNECTED) {
                        vpnRepository.getDnsStats().onSuccess { stats ->
                            _dnsStats.value = stats
                        }
                        delay(60_000L)
                    }
                }
            }
        }
    }

    private fun autoSelectServerIfNeeded() {
        if (vpnRepository.getSelectedServerId() != null) return
        viewModelScope.launch {
            vpnRepository.getServers().onSuccess { servers ->
                if (servers.size == 1 && vpnRepository.getSelectedServerId() == null) {
                    vpnRepository.saveSelectedServer(servers[0].id, servers[0].name)
                    _selectedServer.value = servers[0]
                    _selectedServerName.value = servers[0].toPresentation().title
                }
            }
        }
    }

    fun refreshSelectedServer() {
        val serverId = vpnRepository.getSelectedServerId()
        if (serverId == null) return
        viewModelScope.launch {
            vpnRepository.getServers().onSuccess { servers ->
                val selected = servers.firstOrNull { it.id == serverId }
                _selectedServer.value = selected
                _selectedServerName.value = selected?.toPresentation()?.title
                    ?: vpnRepository.getSelectedServerName()
                    ?: "Servidor automático"
            }
        }
    }

    fun refreshNetworkInfo() {
        viewModelScope.launch {
            vpnRepository.getNetworkInfo().onSuccess { info ->
                _networkInfo.value = info
            }
        }
    }

    fun bindService(service: EscudoVpnService) {
        vpnService = service
        viewModelScope.launch {
            service.connectionState.collect { state ->
                _connectionState.value = state
            }
        }
        viewModelScope.launch {
            service.trafficStats.collect { stats ->
                _trafficStats.value = stats
            }
        }
        viewModelScope.launch {
            service.connectionTime.collect { time ->
                _connectionTime.value = time
            }
        }
    }

    fun unbindService() {
        vpnService = null
    }

    fun connect() {
        val serverId = selectedServerId
        if (serverId == null) {
            _error.value = "Selecione um servidor primeiro"
            return
        }
        val service = vpnService
        if (service == null) {
            _connectionState.value = ConnectionState.DISCONNECTED
            _error.value = "Serviço VPN indisponível. Abra o app novamente e tente conectar."
            return
        }

        viewModelScope.launch {
            _connectionState.value = ConnectionState.CONNECTING
            _error.value = null
            val result = vpnRepository.connect(serverId)
            result.fold(
                onSuccess = { response ->
                    try {
                        android.util.Log.i("EscudoVPN", "Connect API succeeded, parsing config")
                        val config = tunnelManager.parseConfig(response.config)
                        val killSwitch = vpnRepository.isKillSwitchEnabled()
                        android.util.Log.i("EscudoVPN", "Parsed config, handing off to service")
                        service.connect(config, killSwitch)
                        refreshNetworkInfo()
                    } catch (_: Exception) {
                        vpnRepository.disconnect()
                        _connectionState.value = ConnectionState.DISCONNECTED
                        _error.value = "Erro ao configurar a VPN. Tente novamente."
                    }
                },
                onFailure = {
                    _connectionState.value = ConnectionState.DISCONNECTED
                    _error.value =
                        "Não foi possível conectar. Verifique sua conexão e tente novamente."
                    refreshNetworkInfo()
                }
            )
        }
    }

    fun connectPrivateMode() {
        val service = vpnService
        if (service == null) {
            _connectionState.value = ConnectionState.DISCONNECTED
            _error.value = "Serviço VPN indisponível. Abra o app novamente e tente conectar."
            return
        }

        viewModelScope.launch {
            _connectionState.value = ConnectionState.CONNECTING
            _error.value = null
            val result = vpnRepository.connectPrivateMode()
            result.fold(
                onSuccess = { response ->
                    try {
                        android.util.Log.i("EscudoVPN", "Private mode API succeeded, parsing config")
                        val config = tunnelManager.parseConfig(response.config)
                        val killSwitch = vpnRepository.isKillSwitchEnabled()
                        _isPrivateMode.value = true
                        android.util.Log.i("EscudoVPN", "Parsed private config, handing off to service")
                        service.connect(config, killSwitch)
                        refreshNetworkInfo()
                    } catch (_: Exception) {
                        vpnRepository.disconnect()
                        _isPrivateMode.value = false
                        _connectionState.value = ConnectionState.DISCONNECTED
                        _error.value = "Erro ao configurar a VPN. Tente novamente."
                    }
                },
                onFailure = {
                    _connectionState.value = ConnectionState.DISCONNECTED
                    _error.value =
                        "Não foi possível conectar. Verifique sua conexão e tente novamente."
                    refreshNetworkInfo()
                }
            )
        }
    }

    fun disconnect() {
        viewModelScope.launch {
            _isPrivateMode.value = false
            vpnService?.disconnect()
            vpnRepository.disconnect().onFailure {
                _error.value =
                    "VPN local desconectada, mas o servidor nao confirmou o desligamento."
            }
            tunnelManager.clearConfig()
            refreshNetworkInfo()
        }
    }
}

@Composable
fun HomeScreen(
    onOpenServers: () -> Unit,
    auditAction: String? = null,
    viewModel: HomeViewModel = hiltViewModel()
) {
    val context = LocalContext.current
    val connectionState by viewModel.connectionState.collectAsState()
    val trafficStats by viewModel.trafficStats.collectAsState()
    val connectionTime by viewModel.connectionTime.collectAsState()
    val dnsStats by viewModel.dnsStats.collectAsState()
    val networkInfo by viewModel.networkInfo.collectAsState()
    val error by viewModel.error.collectAsState()
    val isPrivateMode by viewModel.isPrivateMode.collectAsState()
    val selectedServerName by viewModel.selectedServerName.collectAsState()
    val selectedServer by viewModel.selectedServer.collectAsState()
    val selectedPresentation = selectedServer?.toPresentation()

    var bound by remember { mutableStateOf(false) }
    var pendingPrivateMode by remember { mutableStateOf(false) }
    var auditActionConsumed by remember(auditAction) { mutableStateOf(false) }

    val vpnPermissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.StartActivityForResult()
    ) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            ContextCompat.startForegroundService(context, Intent(context, EscudoVpnService::class.java))
            if (pendingPrivateMode) {
                pendingPrivateMode = false
                viewModel.connectPrivateMode()
            } else {
                viewModel.connect()
            }
        } else {
            pendingPrivateMode = false
        }
    }

    val serviceConnection = remember {
        object : ServiceConnection {
            override fun onServiceConnected(name: ComponentName?, binder: IBinder?) {
                val service = (binder as? EscudoVpnService.LocalBinder)?.getService() ?: return
                viewModel.bindService(service)
                bound = true
            }

            override fun onServiceDisconnected(name: ComponentName?) {
                viewModel.unbindService()
                bound = false
            }
        }
    }

    LaunchedEffect(Unit) {
        viewModel.refreshSelectedServer()
        viewModel.refreshNetworkInfo()
        val intent = Intent(context, EscudoVpnService::class.java)
        context.bindService(intent, serviceConnection, Context.BIND_AUTO_CREATE)
    }

    DisposableEffect(Unit) {
        onDispose {
            if (bound) {
                try {
                    context.unbindService(serviceConnection)
                } catch (_: Exception) {
                }
            }
        }
    }

    LaunchedEffect(auditAction, auditActionConsumed, bound, connectionState) {
        if (auditActionConsumed || auditAction.isNullOrBlank() || !bound) return@LaunchedEffect

        when (auditAction) {
            com.escudo.vpn.ui.MainActivity.AUDIT_ACTION_CONNECT -> {
                if (connectionState == ConnectionState.DISCONNECTED) {
                    android.util.Log.i("EscudoVPN", "Audit action connect: triggering deterministic connect")
                    viewModel.connect()
                    auditActionConsumed = true
                } else {
                    android.util.Log.i("EscudoVPN", "Audit action connect: already non-disconnected state=$connectionState")
                    auditActionConsumed = true
                }
            }

            com.escudo.vpn.ui.MainActivity.AUDIT_ACTION_DISCONNECT -> {
                if (connectionState == ConnectionState.CONNECTED) {
                    android.util.Log.i("EscudoVPN", "Audit action disconnect: triggering disconnect")
                    viewModel.disconnect()
                }
                auditActionConsumed = true
            }
        }
    }

    Box(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
    ) {
        Image(
            painter = painterResource(id = R.drawable.hero_gold_v3),
            contentDescription = null,
            modifier = Modifier
                .fillMaxSize()
                .alpha(0.08f),
            contentScale = ContentScale.Crop
        )

        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(24.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
            verticalArrangement = Arrangement.spacedBy(24.dp)
        ) {
            Column(
                horizontalAlignment = Alignment.CenterHorizontally,
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = "Escudo",
                    fontWeight = FontWeight.Black,
                    fontSize = 28.sp,
                    color = TextPrimary,
                    letterSpacing = (-0.03).em
                )
                Spacer(modifier = Modifier.height(4.dp))
                Text(
                    text = when (connectionState) {
                        ConnectionState.CONNECTED -> "Protected"
                        ConnectionState.CONNECTING -> "Connecting"
                        ConnectionState.DISCONNECTING -> "Disconnecting"
                        ConnectionState.DISCONNECTED -> "Not connected"
                    },
                    fontSize = 13.sp,
                    color = if (connectionState == ConnectionState.CONNECTED) ConnectedGreen else TextSecondary
                )
            }

            CurrentNetworkCard(
                networkInfo = networkInfo,
                selectedServerName = selectedPresentation?.title ?: selectedServerName,
                selectedExitLabel = selectedPresentation?.category?.exitLabel ?: "Direct route",
                connectionState = connectionState,
                onOpenServers = onOpenServers
            )

            ConnectButton(
                connectionState = connectionState,
                onClick = {
                    when (connectionState) {
                        ConnectionState.DISCONNECTED -> {
                            val vpnIntent = VpnService.prepare(context)
                            if (vpnIntent != null) {
                                vpnPermissionLauncher.launch(vpnIntent)
                            } else {
                                ContextCompat.startForegroundService(
                                    context,
                                    Intent(context, EscudoVpnService::class.java)
                                )
                                viewModel.connect()
                            }
                        }

                        ConnectionState.CONNECTED -> {
                            viewModel.disconnect()
                        }

                        else -> {}
                    }
                }
            )

            Card(
                modifier = Modifier.fillMaxWidth(),
                shape = RoundedCornerShape(16.dp),
                colors = CardDefaults.cardColors(containerColor = CardBackground)
            ) {
                Column(
                    modifier = Modifier.padding(20.dp),
                    verticalArrangement = Arrangement.spacedBy(14.dp)
                ) {
                    Text(
                        text = when (connectionState) {
                            ConnectionState.CONNECTED -> if (isPrivateMode) "Private mode active" else "Secure route active"
                            ConnectionState.CONNECTING -> "Starting secure route"
                            ConnectionState.DISCONNECTING -> "Stopping secure route"
                            ConnectionState.DISCONNECTED -> "Ready to connect"
                        },
                        style = MaterialTheme.typography.headlineMedium,
                        color = when (connectionState) {
                            ConnectionState.CONNECTED -> if (isPrivateMode) AccentDark else ConnectedGreen
                            ConnectionState.CONNECTING -> Accent
                            else -> TextSecondary
                        }
                    )

                    Text(
                        text = when (connectionState) {
                            ConnectionState.CONNECTED -> "Your traffic is encrypted and your selected exit is live."
                            ConnectionState.CONNECTING -> "Escudo is preparing WireGuard, DNS protection, and your exit route."
                            ConnectionState.DISCONNECTING -> "Escudo is closing the current tunnel."
                            ConnectionState.DISCONNECTED -> "Choose a route and tap the power button to go live."
                        },
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary
                    )

                    ServerSummaryCard(
                        title = selectedPresentation?.title ?: selectedServerName,
                        badge = selectedPresentation?.category?.badgeLabel ?: "Direct",
                        exitLabel = selectedPresentation?.category?.exitLabel
                            ?: "Select a route",
                        onOpenServers = onOpenServers
                    )
                }
            }

            Text(
                text = when (connectionState) {
                    ConnectionState.CONNECTED -> "Tap to disconnect"
                    ConnectionState.CONNECTING -> "Connecting now"
                    ConnectionState.DISCONNECTING -> "Disconnecting now"
                    ConnectionState.DISCONNECTED -> "Tap to connect now"
                },
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )

            OutlinedButton(
                onClick = onOpenServers,
                border = BorderStroke(1.dp, Accent.copy(alpha = 0.4f)),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.outlinedButtonColors(contentColor = Accent),
                modifier = Modifier.fillMaxWidth(0.72f)
            ) {
                Text("Change Route")
            }

            if (connectionState == ConnectionState.DISCONNECTED) {
                OutlinedButton(
                    onClick = {
                        val vpnIntent = VpnService.prepare(context)
                        if (vpnIntent != null) {
                            pendingPrivateMode = true
                            vpnPermissionLauncher.launch(vpnIntent)
                        } else {
                            ContextCompat.startForegroundService(
                                context,
                                Intent(context, EscudoVpnService::class.java)
                            )
                            viewModel.connectPrivateMode()
                        }
                    },
                    border = BorderStroke(1.dp, AccentDark.copy(alpha = 0.6f)),
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.outlinedButtonColors(
                        contentColor = AccentDark
                    ),
                    modifier = Modifier.fillMaxWidth(0.72f)
                ) {
                    Row(
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Shield,
                            contentDescription = null,
                            modifier = Modifier.size(20.dp),
                            tint = AccentDark
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text(
                                text = "Private Mode",
                                style = MaterialTheme.typography.titleSmall,
                                color = AccentDark
                            )
                            Text(
                                text = "International route for higher privacy",
                                style = MaterialTheme.typography.bodySmall,
                                color = AccentDark.copy(alpha = 0.7f)
                            )
                        }
                    }
                }
            }

            RealWorldMapCard(
                networkInfo = networkInfo,
                selectedServerName = selectedPresentation?.title ?: selectedServerName,
                selectedServerLocation = selectedPresentation?.subtitle ?: selectedServer?.location,
                connectionState = connectionState
            )

            RouteOverviewCard(
                networkInfo = networkInfo,
                selectedServerName = selectedPresentation?.title ?: selectedServerName,
                selectedExitLabel = selectedPresentation?.category?.exitLabel ?: "Choose a route",
                connectionState = connectionState
            )

            if (connectionState == ConnectionState.CONNECTED) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.Center,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Text("↓ ", style = MonoLabel, color = Accent)
                    Text(formatSpeed(trafficStats.rxBytes), style = MonoLabel, color = Accent)
                    Spacer(Modifier.width(24.dp))
                    Text("↑ ", style = MonoLabel, color = Accent)
                    Text(formatSpeed(trafficStats.txBytes), style = MonoLabel, color = Accent)
                }
            }

            if (error != null) {
                Text(
                text = error!!,
                color = MaterialTheme.colorScheme.error,
                style = MaterialTheme.typography.bodyMedium,
                textAlign = TextAlign.Center
            )
            }

            if (connectionState == ConnectionState.CONNECTED) {
                StatusBar(
                    trafficStats = trafficStats,
                    connectionTimeSecs = connectionTime
                )
                ProtectionSummaryCard(dnsStats = dnsStats)
            } else {
                CurrentNetworkCard(
                    networkInfo = networkInfo,
                    selectedServerName = selectedPresentation?.title ?: selectedServerName,
                    selectedExitLabel = selectedPresentation?.category?.exitLabel ?: "Direct route",
                    connectionState = connectionState,
                    onOpenServers = onOpenServers
                )
            }
        }
    }
}

@Composable
private fun RouteOverviewCard(
    networkInfo: NetworkInfo?,
    selectedServerName: String,
    selectedExitLabel: String,
    connectionState: ConnectionState
) {
    EscudoCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text(
                text = "Route Overview",
                style = MaterialTheme.typography.titleLarge
            )
            Text(
                text = when (connectionState) {
                    ConnectionState.CONNECTED -> "Your VPN route is active and your traffic is protected."
                    ConnectionState.CONNECTING -> "Escudo is preparing the secure tunnel and exit route."
                    ConnectionState.DISCONNECTING -> "Escudo is closing the active route."
                    ConnectionState.DISCONNECTED -> "Choose a route, then connect."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                HighlightFact(networkLocationLabel(networkInfo), "Origin")
                HighlightFact(selectedServerName, "Exit")
                HighlightFact(
                    when (connectionState) {
                        ConnectionState.CONNECTED -> "Active"
                        ConnectionState.CONNECTING -> "Starting"
                        ConnectionState.DISCONNECTING -> "Stopping"
                        ConnectionState.DISCONNECTED -> "Ready"
                    },
                    "State"
                )
            }
            Text(
                text = selectedExitLabel,
                style = MaterialTheme.typography.bodyMedium,
                color = Accent
            )
        }
    }
}

@Composable
private fun RealWorldMapCard(
    networkInfo: NetworkInfo?,
    selectedServerName: String,
    selectedServerLocation: String?,
    connectionState: ConnectionState
) {
    val userMarker = mapUserMarker(networkInfo)
    val serverMarker = mapServerMarker(selectedServerName, selectedServerLocation)

    EscudoCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(verticalArrangement = Arrangement.spacedBy(14.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "World Map",
                        style = MaterialTheme.typography.titleLarge
                    )
                    Text(
                        text = "Your origin and selected VPN exit on a real global map.",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary
                    )
                }
                MapPill(
                    text = when (connectionState) {
                        ConnectionState.CONNECTED -> "LIVE"
                        ConnectionState.CONNECTING -> "SYNC"
                        ConnectionState.DISCONNECTING -> "OFF"
                        ConnectionState.DISCONNECTED -> "READY"
                    }
                )
            }

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(220.dp)
                    .clip(RoundedCornerShape(18.dp))
                    .background(Background)
            ) {
                Image(
                    painter = painterResource(id = R.drawable.world_map_real),
                    contentDescription = "World map",
                    modifier = Modifier
                        .fillMaxSize()
                        .padding(4.dp),
                    contentScale = ContentScale.FillBounds
                )

                Canvas(modifier = Modifier.matchParentSize()) {
                    val width = size.width
                    val height = size.height
                    val routeColor = when (connectionState) {
                        ConnectionState.CONNECTED -> ConnectedGreen
                        ConnectionState.CONNECTING -> Accent
                        else -> Accent.copy(alpha = 0.45f)
                    }

                    if (userMarker != null && serverMarker != null) {
                        drawLine(
                            color = routeColor.copy(alpha = 0.7f),
                            start = Offset(userMarker.x * width, userMarker.y * height),
                            end = Offset(serverMarker.x * width, serverMarker.y * height),
                            strokeWidth = 4f
                        )
                    }

                    listOfNotNull(userMarker, serverMarker).forEach { marker ->
                        val center = Offset(marker.x * width, marker.y * height)
                        val dotColor = if (marker.kind == MapMarkerKind.User) AccentDark else routeColor
                        drawCircle(
                            color = dotColor.copy(alpha = 0.18f),
                            radius = 18f,
                            center = center
                        )
                        drawCircle(
                            color = dotColor,
                            radius = 7f,
                            center = center
                        )
                    }
                }

                if (userMarker != null) {
                    MapLabel(
                        text = "You",
                        modifier = Modifier
                            .align(Alignment.TopStart)
                            .padding(start = 12.dp, top = 12.dp)
                    )
                }
                if (serverMarker != null) {
                    MapLabel(
                        text = selectedServerName,
                        modifier = Modifier
                            .align(Alignment.BottomEnd)
                            .padding(end = 12.dp, bottom = 12.dp)
                    )
                }
            }
        }
    }
}

@Composable
private fun ServerSummaryCard(
        title: String,
        badge: String,
        exitLabel: String,
        onOpenServers: () -> Unit
) {
    Card(
        modifier = Modifier
            .fillMaxWidth()
            .clickable(onClick = onOpenServers),
        shape = RoundedCornerShape(20.dp),
        colors = CardDefaults.cardColors(containerColor = Background.copy(alpha = 0.35f))
    ) {
        Row(
            modifier = Modifier
                .fillMaxWidth()
                .padding(16.dp),
            verticalAlignment = Alignment.CenterVertically
        ) {
            Box(
                modifier = Modifier
                    .size(42.dp)
                    .clip(CircleShape)
                    .background(Accent.copy(alpha = 0.12f)),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Public,
                    contentDescription = null,
                    tint = Accent
                )
            }
            Spacer(modifier = Modifier.width(12.dp))
            Column(modifier = Modifier.weight(1f)) {
                Text(
                    text = title,
                    style = MaterialTheme.typography.titleMedium
                )
                Text(
                    text = badge,
                    style = MaterialTheme.typography.labelMedium,
                    color = Accent
                )
                Text(
                    text = exitLabel,
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextSecondary
                )
            }
            Icon(
                imageVector = Icons.Default.KeyboardArrowRight,
                contentDescription = "Choose route",
                tint = TextSecondary
            )
        }
    }
}

@Composable
private fun CurrentNetworkCard(
    networkInfo: NetworkInfo?,
    selectedServerName: String,
    selectedExitLabel: String,
    connectionState: ConnectionState,
    onOpenServers: () -> Unit
) {
    EscudoCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            Text(
                text = "Current Network",
                style = MaterialTheme.typography.titleLarge
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                HighlightFact(networkInfo?.ip ?: "--", "IP")
                HighlightFact(networkLocationLabel(networkInfo), "Origin")
                HighlightFact(selectedServerName, if (connectionState == ConnectionState.CONNECTED) "Exit" else "Next exit")
            }
            Text(
                text = when (connectionState) {
                    ConnectionState.CONNECTED -> "Connected now. Your live IP is shown above and traffic exits through $selectedExitLabel."
                    ConnectionState.CONNECTING -> "Escudo is bringing your route up now."
                    ConnectionState.DISCONNECTING -> "Escudo is closing the current tunnel."
                    ConnectionState.DISCONNECTED -> "Your current public IP is shown above. Pick a route, then connect."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )
            Spacer(modifier = Modifier.height(12.dp))
            OutlinedButton(
                onClick = onOpenServers,
                border = BorderStroke(1.dp, Accent.copy(alpha = 0.4f)),
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.outlinedButtonColors(contentColor = Accent),
                modifier = Modifier.fillMaxWidth()
            ) {
                Text("Choose Route")
            }
        }
    }
}

@Composable
private fun ProtectionSummaryCard(
    dnsStats: DnsStats
) {
    EscudoCard(
        modifier = Modifier.fillMaxWidth()
    ) {
        Column(
            verticalArrangement = Arrangement.spacedBy(14.dp)
        ) {
            Text(
                text = "Protection",
                style = MaterialTheme.typography.titleLarge
            )
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween
            ) {
                HighlightFact(dnsStats.blockedToday.toString(), "Blocked today")
                HighlightFact(dnsStats.blockedAllTime.toString(), "Blocked total")
                HighlightFact(dnsStats.queriesToday.toString(), "Queries")
            }
            Text(
                text = "Ads, malware, and trackers remain filtered while the VPN is active.",
                style = MaterialTheme.typography.bodyMedium,
                color = ConnectedGreen
            )
        }
    }
}

@Composable
private fun HighlightFact(
    value: String,
    label: String
) {
    Column(horizontalAlignment = Alignment.CenterHorizontally) {
        Text(
            text = value,
            style = MaterialTheme.typography.titleLarge,
            color = Accent,
            textAlign = TextAlign.Center
        )
        Text(
            text = label,
            style = MaterialTheme.typography.labelMedium,
            color = TextSecondary
        )
    }
}

private fun networkLocationLabel(info: NetworkInfo?): String {
    if (info == null) return "--"
    val city = info.city?.takeIf { it.isNotBlank() }
    val country = info.country?.takeIf { it.isNotBlank() }
    return listOfNotNull(city, country).joinToString(", ").ifBlank { country ?: "--" }
}

private enum class MapMarkerKind {
    User,
    Server
}

private data class MapMarker(
    val x: Float,
    val y: Float,
    val kind: MapMarkerKind
)

@Composable
private fun MapLabel(
    text: String,
    modifier: Modifier = Modifier
) {
    Box(
        modifier = modifier
            .clip(RoundedCornerShape(999.dp))
            .background(Background.copy(alpha = 0.94f))
            .padding(horizontal = 10.dp, vertical = 6.dp)
    ) {
        Text(
            text = text,
            style = MaterialTheme.typography.labelMedium,
            color = TextPrimary
        )
    }
}

@Composable
private fun MapPill(
    text: String
) {
    Box(
        modifier = Modifier
            .clip(RoundedCornerShape(999.dp))
            .background(Accent.copy(alpha = 0.12f))
            .padding(horizontal = 10.dp, vertical = 6.dp)
    ) {
        Text(
            text = text,
            style = MonoLabel,
            color = AccentDark
        )
    }
}

private fun androidx.compose.ui.graphics.drawscope.DrawScope.drawMapBlob(
    width: Float,
    height: Float,
    points: List<Offset>
) {
    val path = androidx.compose.ui.graphics.Path().apply {
        if (points.isNotEmpty()) {
            moveTo(points.first().x * width, points.first().y * height)
            for (index in 1 until points.size) {
                val point = points[index]
                lineTo(point.x * width, point.y * height)
            }
            close()
        }
    }
    drawPath(path = path, color = TextPrimary.copy(alpha = 0.06f))
}

private fun mapServerMarker(serverName: String, serverLocation: String?): MapMarker? {
    val key = listOf(serverName, serverLocation.orEmpty()).joinToString(" ").lowercase()
    return when {
        "são paulo" in key || "sao paulo" in key -> MapMarker(0.31f, 0.67f, MapMarkerKind.Server)
        "new jersey" in key || "ashburn" in key -> MapMarker(0.23f, 0.34f, MapMarkerKind.Server)
        "hillsboro" in key -> MapMarker(0.16f, 0.28f, MapMarkerKind.Server)
        "london" in key -> MapMarker(0.49f, 0.28f, MapMarkerKind.Server)
        "amsterdam" in key -> MapMarker(0.52f, 0.28f, MapMarkerKind.Server)
        "helsinki" in key -> MapMarker(0.58f, 0.22f, MapMarkerKind.Server)
        "falkenstein" in key || "nuremberg" in key -> MapMarker(0.55f, 0.30f, MapMarkerKind.Server)
        "singapore" in key -> MapMarker(0.76f, 0.57f, MapMarkerKind.Server)
        "sydney" in key -> MapMarker(0.84f, 0.78f, MapMarkerKind.Server)
        "toronto" in key -> MapMarker(0.20f, 0.25f, MapMarkerKind.Server)
        "bangalore" in key -> MapMarker(0.67f, 0.50f, MapMarkerKind.Server)
        else -> null
    }
}

private fun mapUserMarker(networkInfo: NetworkInfo?): MapMarker? {
    val key = buildString {
        append(networkInfo?.city.orEmpty())
        append(" ")
        append(networkInfo?.country.orEmpty())
        append(" ")
        append(networkInfo?.countryCode.orEmpty())
    }.lowercase()
    return when {
        "brazil" in key || "brasil" in key || "são paulo" in key || "sao paulo" in key || " br " in key -> MapMarker(0.34f, 0.60f, MapMarkerKind.User)
        "united states" in key || "estados unidos" in key || "new york" in key || "new jersey" in key || " us " in key -> MapMarker(0.22f, 0.30f, MapMarkerKind.User)
        "canada" in key || "toronto" in key || " ca " in key -> MapMarker(0.20f, 0.24f, MapMarkerKind.User)
        "united kingdom" in key || "reino unido" in key || "london" in key || " gb " in key || " uk " in key -> MapMarker(0.49f, 0.27f, MapMarkerKind.User)
        "germany" in key || "alemanha" in key || " de " in key -> MapMarker(0.54f, 0.30f, MapMarkerKind.User)
        "finland" in key || "helsinki" in key || " fi " in key -> MapMarker(0.58f, 0.22f, MapMarkerKind.User)
        "singapore" in key -> MapMarker(0.76f, 0.57f, MapMarkerKind.User)
        "australia" in key || "sydney" in key || " au " in key -> MapMarker(0.84f, 0.79f, MapMarkerKind.User)
        "india" in key || "bangalore" in key -> MapMarker(0.67f, 0.49f, MapMarkerKind.User)
        else -> null
    }
}

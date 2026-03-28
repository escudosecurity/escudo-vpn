package com.escudo.vpn.ui.screens

import android.Manifest
import android.app.Application
import android.content.Context
import android.content.Intent
import android.webkit.WebView
import android.webkit.WebViewClient
import android.graphics.Color as AndroidColor
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.AccountCircle
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Close
import androidx.compose.material.icons.filled.Lock
import androidx.compose.material.icons.filled.Logout
import androidx.compose.material.icons.filled.Security
import androidx.compose.material.icons.filled.Shield
import androidx.compose.material.icons.filled.Wifi
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.Divider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Switch
import androidx.compose.material3.SwitchDefaults
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.viewinterop.AndroidView
import androidx.core.content.ContextCompat
import androidx.hilt.navigation.compose.hiltViewModel
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import com.escudo.vpn.data.model.DnsStats
import com.escudo.vpn.data.model.LaunchStatusResponse
import com.escudo.vpn.data.model.PairQrResponse
import com.escudo.vpn.data.prefs.SecurePrefs
import com.escudo.vpn.data.repository.AuthRepository
import com.escudo.vpn.data.repository.VpnRepository
import com.escudo.vpn.service.WifiSsidResolver
import com.escudo.vpn.ui.theme.Accent
import com.escudo.vpn.ui.theme.Background
import com.escudo.vpn.ui.theme.CardBackground
import com.escudo.vpn.ui.theme.DividerColor
import com.escudo.vpn.ui.theme.ErrorRed
import com.escudo.vpn.ui.theme.MonoLabel
import com.escudo.vpn.ui.theme.TextPrimary
import com.escudo.vpn.ui.theme.TextSecondary
import dagger.hilt.android.lifecycle.HiltViewModel
import javax.inject.Inject
import java.net.URLEncoder
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asSharedFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

@HiltViewModel
class SettingsViewModel @Inject constructor(
    application: Application,
    private val authRepository: AuthRepository,
    private val vpnRepository: VpnRepository,
    private val securePrefs: SecurePrefs
) : AndroidViewModel(application) {

    private val _killSwitchEnabled = MutableStateFlow(vpnRepository.isKillSwitchEnabled())
    val killSwitchEnabled: StateFlow<Boolean> = _killSwitchEnabled.asStateFlow()

    private val _autoProtectEnabled = MutableStateFlow(securePrefs.isAutoProtectEnabled())
    val autoProtectEnabled: StateFlow<Boolean> = _autoProtectEnabled.asStateFlow()

    private val _hasWifiPermission =
        MutableStateFlow(WifiSsidResolver.hasRequiredPermission(application))
    val hasWifiPermission: StateFlow<Boolean> = _hasWifiPermission.asStateFlow()

    private val _trustedNetworks = MutableStateFlow(securePrefs.getTrustedNetworks())
    val trustedNetworks: StateFlow<Set<String>> = _trustedNetworks.asStateFlow()

    private val _dnsStats = MutableStateFlow(DnsStats())
    val dnsStats: StateFlow<DnsStats> = _dnsStats.asStateFlow()

    private val _logoutInProgress = MutableStateFlow(false)
    val logoutInProgress: StateFlow<Boolean> = _logoutInProgress.asStateFlow()

    private val _logoutError = MutableStateFlow<String?>(null)
    val logoutError: StateFlow<String?> = _logoutError.asStateFlow()

    private val _logoutCompleted = MutableSharedFlow<Unit>(extraBufferCapacity = 1)
    val logoutCompleted: SharedFlow<Unit> = _logoutCompleted.asSharedFlow()

    private val _pairQr = MutableStateFlow<PairQrResponse?>(null)
    val pairQr: StateFlow<PairQrResponse?> = _pairQr.asStateFlow()

    private val _launchStatus = MutableStateFlow<LaunchStatusResponse?>(null)
    val launchStatus: StateFlow<LaunchStatusResponse?> = _launchStatus.asStateFlow()

    val userEmail: String
        get() = authRepository.getUserEmail() ?: ""

    val accountCode: String
        get() = authRepository.getAccountCode() ?: ""

    init {
        refreshLaunchStatus()
        viewModelScope.launch {
            while (true) {
                vpnRepository.getDnsStats().onSuccess { _dnsStats.value = it }
                delay(60_000L)
            }
        }
    }

    fun setKillSwitch(enabled: Boolean) {
        vpnRepository.setKillSwitch(enabled)
        _killSwitchEnabled.value = enabled
        val app = getApplication<Application>()
        val intent = Intent(app, com.escudo.vpn.service.EscudoVpnService::class.java).apply {
            putExtra(
                if (enabled) com.escudo.vpn.service.EscudoVpnService.EXTRA_ENFORCE_KILL_SWITCH
                else com.escudo.vpn.service.EscudoVpnService.EXTRA_DISABLE_KILL_SWITCH,
                true
            )
        }

        if (enabled) {
            if (android.net.VpnService.prepare(app) == null) {
                ContextCompat.startForegroundService(app, intent)
            }
        } else {
            app.startService(intent)
        }
    }

    fun setAutoProtect(enabled: Boolean) {
        securePrefs.setAutoProtectEnabled(enabled)
        _autoProtectEnabled.value = enabled
    }

    fun refreshWifiPermission() {
        _hasWifiPermission.value = WifiSsidResolver.hasRequiredPermission(getApplication())
    }

    fun addTrustedNetwork(ssid: String) {
        securePrefs.addTrustedNetwork(ssid)
        _trustedNetworks.value = securePrefs.getTrustedNetworks()
    }

    fun removeTrustedNetwork(ssid: String) {
        securePrefs.removeTrustedNetwork(ssid)
        _trustedNetworks.value = securePrefs.getTrustedNetworks()
    }

    fun logout() {
        if (_logoutInProgress.value) return

        viewModelScope.launch {
            _logoutInProgress.value = true
            _logoutError.value = null

            vpnRepository.disconnect().fold(
                onSuccess = {
                    val intent = Intent(
                        getApplication(),
                        com.escudo.vpn.service.EscudoVpnService::class.java
                    )
                    getApplication<Application>().stopService(intent)
                    authRepository.logout()
                    _logoutCompleted.tryEmit(Unit)
                },
                onFailure = {
                    _logoutError.value =
                        "Nao foi possivel encerrar a sessao VPN no servidor. Tente novamente."
                }
            )

            _logoutInProgress.value = false
        }
    }

    fun generatePairQr() {
        viewModelScope.launch {
            authRepository.generatePairQr().onSuccess {
                _pairQr.value = it
            }
        }
    }

    fun refreshLaunchStatus() {
        viewModelScope.launch {
            authRepository.getLaunchStatus().onSuccess {
                _launchStatus.value = it
            }
        }
    }
}

@Composable
fun ProtectionScreen(
    viewModel: SettingsViewModel = hiltViewModel()
) {
    val context = LocalContext.current
    val killSwitchEnabled by viewModel.killSwitchEnabled.collectAsState()
    val autoProtectEnabled by viewModel.autoProtectEnabled.collectAsState()
    val hasWifiPermission by viewModel.hasWifiPermission.collectAsState()
    val trustedNetworks by viewModel.trustedNetworks.collectAsState()
    val dnsStats by viewModel.dnsStats.collectAsState()
    val locationPermissionLauncher = rememberLauncherForActivityResult(
        contract = ActivityResultContracts.RequestPermission()
    ) { granted ->
        viewModel.refreshWifiPermission()
        if (granted) {
            viewModel.setAutoProtect(true)
        }
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
            .padding(20.dp)
            .verticalScroll(rememberScrollState())
    ) {
        Text(
            text = "Proteção",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Bloqueio, kill switch e Wi-Fi seguro para manter o app pronto todos os dias.",
            style = MaterialTheme.typography.bodyMedium,
            color = TextSecondary
        )

        Spacer(modifier = Modifier.height(24.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(containerColor = CardBackground)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Escudo ativo",
                    style = MaterialTheme.typography.titleLarge
                )
                Spacer(modifier = Modifier.height(12.dp))
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = androidx.compose.foundation.layout.Arrangement.SpaceEvenly,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    SecurityFact("Hoje", dnsStats.blockedToday.toString())
                    SecurityFact("Total", dnsStats.blockedAllTime.toString())
                    SecurityFact("Consultas", dnsStats.queriesToday.toString())
                }
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = "Ads, malware e trackers seguem filtrados pelo DNS seguro do Escudo.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = Accent
                )
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(containerColor = CardBackground)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Icon(
                        imageVector = Icons.Default.Security,
                        contentDescription = null,
                        tint = Accent,
                        modifier = Modifier.size(24.dp)
                    )
                    Spacer(modifier = Modifier.width(12.dp))
                    Column(modifier = Modifier.weight(1f)) {
                        Text(
                            text = "Kill switch",
                            style = MaterialTheme.typography.titleMedium
                        )
                        Text(
                            text = "Bloqueia a internet até a VPN reconectar ou você desligar a proteção.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = TextSecondary
                        )
                    }
                    Switch(
                        checked = killSwitchEnabled,
                        onCheckedChange = { viewModel.setKillSwitch(it) },
                        colors = SwitchDefaults.colors(
                            checkedThumbColor = Accent,
                            checkedTrackColor = Accent.copy(alpha = 0.3f)
                        )
                    )
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        WifiProtectionSection(
            autoProtectEnabled = autoProtectEnabled,
            hasWifiPermission = hasWifiPermission,
            trustedNetworks = trustedNetworks,
            onAutoProtectChanged = {
                if (it && !hasWifiPermission) {
                    locationPermissionLauncher.launch(Manifest.permission.ACCESS_FINE_LOCATION)
                } else {
                    viewModel.setAutoProtect(it)
                }
            },
            onAddCurrentNetwork = {
                if (!hasWifiPermission) {
                    locationPermissionLauncher.launch(Manifest.permission.ACCESS_FINE_LOCATION)
                } else {
                    val ssid = getCurrentWifiSsid(context)
                    if (ssid != null) {
                        viewModel.addTrustedNetwork(ssid)
                    }
                }
            },
            onRemoveNetwork = { viewModel.removeTrustedNetwork(it) },
            currentSsid = getCurrentWifiSsid(context),
            onRequestLocationPermission = {
                locationPermissionLauncher.launch(Manifest.permission.ACCESS_FINE_LOCATION)
            }
        )

        Spacer(modifier = Modifier.height(16.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(containerColor = CardBackground)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Text(
                    text = "Filtros do app",
                    style = MaterialTheme.typography.titleLarge
                )
                Spacer(modifier = Modifier.height(12.dp))
                ProtectionHint("Ad Block", "Ativo em toda a rede")
                Spacer(modifier = Modifier.height(8.dp))
                ProtectionHint("Malware Block", "Ativo em toda a rede")
                Spacer(modifier = Modifier.height(8.dp))
                ProtectionHint("Family Mode", "Gerencie perfis infantis na aba Família")
                Spacer(modifier = Modifier.height(8.dp))
                ProtectionHint("Dark Web Monitor", "Integração visual vem no próximo update")
            }
        }
    }
}

@Composable
fun AccountScreen(
    onLogout: () -> Unit,
    viewModel: SettingsViewModel = hiltViewModel()
) {
    val clipboard = LocalClipboardManager.current
    val context = LocalContext.current
    val logoutInProgress by viewModel.logoutInProgress.collectAsState()
    val logoutError by viewModel.logoutError.collectAsState()
    val pairQr by viewModel.pairQr.collectAsState()
    val launchStatus by viewModel.launchStatus.collectAsState()
    var showLogoutDialog by remember { mutableStateOf(false) }

    androidx.compose.runtime.LaunchedEffect(Unit) {
        viewModel.logoutCompleted.collect {
            onLogout()
        }
    }

    if (showLogoutDialog) {
        AlertDialog(
            onDismissRequest = { showLogoutDialog = false },
            title = { Text("Sair da conta") },
            text = { Text("Tem certeza que deseja sair? A VPN será desconectada.") },
            confirmButton = {
                TextButton(
                    enabled = !logoutInProgress,
                    onClick = {
                        showLogoutDialog = false
                        viewModel.logout()
                    }
                ) {
                    Text("Sair", color = ErrorRed)
                }
            },
            dismissButton = {
                TextButton(
                    enabled = !logoutInProgress,
                    onClick = { showLogoutDialog = false }
                ) {
                    Text("Cancelar")
                }
            }
        )
    }

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(Background)
            .padding(20.dp)
            .verticalScroll(rememberScrollState())
    ) {
        Text(
            text = "Conta",
            style = MaterialTheme.typography.headlineMedium
        )
        Spacer(modifier = Modifier.height(8.dp))
        Text(
            text = "Manage your account code, device access, and pairing tools.",
            style = MaterialTheme.typography.bodyMedium,
            color = TextSecondary
        )

        Spacer(modifier = Modifier.height(24.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(containerColor = CardBackground)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                SettingsRow(
                    icon = Icons.Default.AccountCircle,
                    title = "Conta conectada",
                    subtitle = viewModel.userEmail.ifBlank { "Conta por codigo" }
                )
                Spacer(modifier = Modifier.height(16.dp))
                TierStatusCard(
                    tier = launchStatus?.effectiveTier,
                    activeInvites = launchStatus?.activeInvites ?: 0L
                )
                Spacer(modifier = Modifier.height(16.dp))
                Text(
                    text = "Codigo da conta",
                    style = MaterialTheme.typography.titleSmall,
                    color = TextSecondary
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = viewModel.accountCode.ifBlank { "No code available" },
                    style = MonoLabel,
                    color = Accent
                )
                Spacer(modifier = Modifier.height(8.dp))
                Text(
                    text = "Use estes 16 digitos para entrar em outro dispositivo.",
                    style = MaterialTheme.typography.bodyMedium,
                    color = TextPrimary
                )
                Spacer(modifier = Modifier.height(12.dp))
                Button(
                    onClick = {
                        clipboard.setText(AnnotatedString(viewModel.accountCode))
                        android.widget.Toast.makeText(context, "Código copiado", android.widget.Toast.LENGTH_SHORT).show()
                    },
                    enabled = viewModel.accountCode.isNotBlank(),
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Text("Copiar código")
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        Card(
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(18.dp),
            colors = CardDefaults.cardColors(containerColor = CardBackground)
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                SettingsRow(
                    icon = Icons.Default.Add,
                    title = "Adicionar dispositivo",
                    subtitle = "Use o codigo da conta, um link, ou escaneie o QR"
                )
                Spacer(modifier = Modifier.height(16.dp))
                Button(
                    onClick = { viewModel.generatePairQr() },
                    shape = RoundedCornerShape(14.dp)
                ) {
                    Text("Gerar QR e link")
                }
                Spacer(modifier = Modifier.height(8.dp))
                ProtectionHint("Entrar em outro dispositivo", "Abra o Escudo no novo dispositivo e use o mesmo codigo ou o QR abaixo.")
                if (pairQr != null) {
                    Spacer(modifier = Modifier.height(8.dp))
                    PairQrCard(pairQr = pairQr!!)
                    Spacer(modifier = Modifier.height(12.dp))
                    Text(
                        text = pairQr!!.qrUrl,
                        style = MonoLabel,
                        color = Accent
                    )
                    Spacer(modifier = Modifier.height(8.dp))
                    Button(
                        onClick = {
                            clipboard.setText(AnnotatedString(pairQr!!.qrUrl))
                            android.widget.Toast.makeText(context, "Link copiado", android.widget.Toast.LENGTH_SHORT).show()
                        },
                        shape = RoundedCornerShape(14.dp)
                    ) {
                        Text("Copiar link")
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        if (logoutError != null) {
            Text(
                text = logoutError!!,
                color = ErrorRed,
                style = MaterialTheme.typography.bodyMedium
            )
            Spacer(modifier = Modifier.height(12.dp))
        }

        Button(
            enabled = !logoutInProgress,
            onClick = { showLogoutDialog = true },
            modifier = Modifier.fillMaxWidth(),
            shape = RoundedCornerShape(16.dp),
            colors = androidx.compose.material3.ButtonDefaults.buttonColors(
                containerColor = ErrorRed,
                contentColor = Background
            )
        ) {
            Icon(
                imageVector = Icons.Default.Logout,
                contentDescription = null,
                modifier = Modifier.size(18.dp)
            )
            Spacer(modifier = Modifier.width(8.dp))
            Text("Sair da conta")
        }
    }
}

@Composable
private fun TierStatusCard(
    tier: String?,
    activeInvites: Long
) {
    val normalizedTier = tier.orEmpty().trim().lowercase()
    val planLabel = when (normalizedTier) {
        "free" -> "Free"
        "escudo" -> "Escudo"
        "pro" -> "Power / Family"
        "dedicated" -> "Dedicated"
        else -> "Free"
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(16.dp),
        colors = CardDefaults.cardColors(containerColor = Background.copy(alpha = 0.55f))
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Text(
                text = "Plano atual",
                style = MaterialTheme.typography.labelLarge,
                color = TextSecondary
            )
            Spacer(modifier = Modifier.height(6.dp))
            Text(
                text = planLabel,
                style = MaterialTheme.typography.titleLarge,
                color = Accent,
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = if (activeInvites > 0) {
                    "$activeInvites codigo(s) premium aplicado(s) nesta conta"
                } else {
                    "Este e o plano ativo desta conta agora."
                },
                style = MaterialTheme.typography.bodyMedium,
                color = TextPrimary
            )
        }
    }
}

@Composable
private fun PairQrCard(
    pairQr: PairQrResponse
) {
    val qrImageUrl = remember(pairQr.qrUrl) {
        val encoded = URLEncoder.encode(pairQr.qrUrl, "UTF-8")
        "https://api.qrserver.com/v1/create-qr-code/?size=512x512&data=$encoded"
    }
    val expiryLabel = remember(pairQr.expiresAt) {
        pairQr.expiresAt.replace("T", " ").replace("Z", " UTC")
    }

    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = Background.copy(alpha = 0.65f))
    ) {
        Column(
            modifier = Modifier.padding(16.dp),
            horizontalAlignment = Alignment.CenterHorizontally
        ) {
            Text(
                text = "Escaneie para adicionar outro dispositivo",
                style = MaterialTheme.typography.titleMedium
            )
            Spacer(modifier = Modifier.height(12.dp))
            AndroidView(
                factory = { context ->
                    WebView(context).apply {
                        setBackgroundColor(AndroidColor.WHITE)
                        settings.javaScriptEnabled = false
                        webViewClient = WebViewClient()
                        loadUrl(qrImageUrl)
                    }
                },
                update = { webView ->
                    webView.loadUrl(qrImageUrl)
                },
                modifier = Modifier
                    .size(220.dp)
                    .background(androidx.compose.ui.graphics.Color.White, RoundedCornerShape(18.dp))
                    .padding(8.dp)
            )
            Spacer(modifier = Modifier.height(12.dp))
            Text(
                text = "Valido ate $expiryLabel",
                style = MaterialTheme.typography.bodySmall,
                color = TextSecondary
            )
        }
    }
}

@Composable
private fun WifiProtectionSection(
    autoProtectEnabled: Boolean,
    hasWifiPermission: Boolean,
    trustedNetworks: Set<String>,
    onAutoProtectChanged: (Boolean) -> Unit,
    onAddCurrentNetwork: () -> Unit,
    onRemoveNetwork: (String) -> Unit,
    currentSsid: String?,
    onRequestLocationPermission: () -> Unit
) {
    Card(
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(18.dp),
        colors = CardDefaults.cardColors(containerColor = CardBackground)
    ) {
        Column(modifier = Modifier.padding(16.dp)) {
            Row(
                modifier = Modifier.fillMaxWidth(),
                verticalAlignment = Alignment.CenterVertically
            ) {
                Icon(
                    imageVector = Icons.Default.Wifi,
                    contentDescription = null,
                    tint = Accent,
                    modifier = Modifier.size(24.dp)
                )
                Spacer(modifier = Modifier.width(12.dp))
                Column(modifier = Modifier.weight(1f)) {
                    Text(
                        text = "Proteção Wi-Fi",
                        style = MaterialTheme.typography.titleMedium
                    )
                    Text(
                        text = "Proteção automática em redes públicas",
                        style = MaterialTheme.typography.bodyMedium,
                        color = TextSecondary
                    )
                }
                Switch(
                    checked = autoProtectEnabled,
                    onCheckedChange = onAutoProtectChanged,
                    colors = SwitchDefaults.colors(
                        checkedThumbColor = Accent,
                        checkedTrackColor = Accent.copy(alpha = 0.3f)
                    )
                )
            }

            Spacer(modifier = Modifier.height(8.dp))

            Text(
                text = "O Escudo será ativado automaticamente ao conectar em redes Wi-Fi não confiáveis.",
                style = MaterialTheme.typography.bodySmall,
                color = TextSecondary
            )

            if (autoProtectEnabled && !hasWifiPermission) {
                Spacer(modifier = Modifier.height(12.dp))
                Text(
                    text = "Permita acesso à localização para identificar a rede Wi-Fi atual e aplicar a proteção automática.",
                    style = MaterialTheme.typography.bodySmall,
                    color = ErrorRed
                )
                Spacer(modifier = Modifier.height(8.dp))
                Button(onClick = onRequestLocationPermission) {
                    Text("Permitir localização")
                }
            }

            if (autoProtectEnabled && hasWifiPermission) {
                Spacer(modifier = Modifier.height(12.dp))
                Divider(color = DividerColor)
                Spacer(modifier = Modifier.height(12.dp))

                Text(
                    text = "Redes confiáveis",
                    style = MaterialTheme.typography.titleSmall,
                    color = TextSecondary
                )

                Spacer(modifier = Modifier.height(8.dp))

                if (trustedNetworks.isEmpty()) {
                    Text(
                        text = "Nenhuma rede confiável adicionada",
                        style = MaterialTheme.typography.bodySmall,
                        color = TextSecondary
                    )
                } else {
                    trustedNetworks.sorted().forEach { ssid ->
                        Row(
                            modifier = Modifier
                                .fillMaxWidth()
                                .padding(vertical = 4.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = Icons.Default.Wifi,
                                contentDescription = null,
                                tint = TextSecondary,
                                modifier = Modifier.size(18.dp)
                            )
                            Spacer(modifier = Modifier.width(8.dp))
                            Text(
                                text = ssid,
                                style = MaterialTheme.typography.bodyMedium,
                                modifier = Modifier.weight(1f)
                            )
                            IconButton(
                                onClick = { onRemoveNetwork(ssid) },
                                modifier = Modifier.size(32.dp)
                            ) {
                                Icon(
                                    imageVector = Icons.Default.Close,
                                    contentDescription = "Remover $ssid",
                                    tint = ErrorRed,
                                    modifier = Modifier.size(18.dp)
                                )
                            }
                        }
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))

                val canAddCurrent = currentSsid != null && !trustedNetworks.contains(currentSsid)
                if (canAddCurrent) {
                    TextButton(onClick = onAddCurrentNetwork) {
                        Icon(
                            imageVector = Icons.Default.Add,
                            contentDescription = null,
                            tint = Accent,
                            modifier = Modifier.size(18.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(
                            text = "Adicionar rede atual ($currentSsid)",
                            color = Accent,
                            style = MaterialTheme.typography.labelLarge
                        )
                    }
                }
            }
        }
    }
}

private fun getCurrentWifiSsid(context: Context): String? {
    return WifiSsidResolver.getCurrentSsid(context)
}

@Composable
private fun SettingsRow(
    icon: ImageVector,
    title: String,
    subtitle: String
) {
    Row(
        modifier = Modifier.fillMaxWidth(),
        verticalAlignment = Alignment.CenterVertically
    ) {
        Icon(
            imageVector = icon,
            contentDescription = null,
            tint = Accent,
            modifier = Modifier.size(24.dp)
        )
        Spacer(modifier = Modifier.width(12.dp))
        Column {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )
        }
    }
}

@Composable
private fun SecurityFact(
    label: String,
    value: String
) {
    Column(
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
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
private fun ProtectionHint(
    title: String,
    subtitle: String
) {
    Row(
        modifier = Modifier
            .fillMaxWidth()
            .border(1.dp, DividerColor, RoundedCornerShape(14.dp))
            .padding(12.dp),
        verticalAlignment = Alignment.CenterVertically
    ) {
        androidx.compose.foundation.layout.Box(
            modifier = Modifier
                .size(34.dp)
                .background(Accent.copy(alpha = 0.12f), CircleShape),
            contentAlignment = Alignment.Center
        ) {
            Icon(
                imageVector = Icons.Default.Lock,
                contentDescription = null,
                tint = Accent,
                modifier = Modifier.size(16.dp)
            )
        }
        Spacer(modifier = Modifier.width(12.dp))
        Column {
            Text(
                text = title,
                style = MaterialTheme.typography.titleMedium
            )
            Text(
                text = subtitle,
                style = MaterialTheme.typography.bodyMedium,
                color = TextSecondary
            )
        }
    }
}

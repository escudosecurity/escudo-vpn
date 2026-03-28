package com.escudo.vpn.service

import android.app.Notification
import android.app.PendingIntent
import android.content.Intent
import android.net.VpnService
import android.os.Binder
import android.os.IBinder
import android.os.ParcelFileDescriptor
import androidx.core.app.NotificationCompat
import com.escudo.vpn.EscudoApp
import com.escudo.vpn.BuildConfig
import com.escudo.vpn.R
import com.escudo.vpn.data.model.ConnectionState
import com.escudo.vpn.data.model.TrafficStats
import com.escudo.vpn.ui.MainActivity
import com.wireguard.android.backend.GoBackend
import com.wireguard.android.backend.Tunnel
import com.wireguard.config.Config
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.cancel
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.isActive
import kotlinx.coroutines.launch

class EscudoVpnService : VpnService() {

    private val binder = LocalBinder()
    private val serviceScope = CoroutineScope(SupervisorJob() + Dispatchers.IO)
    private var statsJob: Job? = null
    private var reconnectJob: Job? = null
    private var connectionStartTime: Long = 0L
    private var backend: GoBackend? = null
    private var tunnel: EscudoTunnel? = null
    private var activeConfig: Config? = null
    private var killSwitchEnabled: Boolean = false
    private var manualDisconnectRequested: Boolean = false
    private var killSwitchInterface: ParcelFileDescriptor? = null

    private val _connectionState = MutableStateFlow(ConnectionState.DISCONNECTED)
    val connectionState: StateFlow<ConnectionState> = _connectionState.asStateFlow()

    private val _trafficStats = MutableStateFlow(TrafficStats())
    val trafficStats: StateFlow<TrafficStats> = _trafficStats.asStateFlow()

    private val _connectionTime = MutableStateFlow(0L)
    val connectionTime: StateFlow<Long> = _connectionTime.asStateFlow()

    inner class LocalBinder : Binder() {
        fun getService(): EscudoVpnService = this@EscudoVpnService
    }

    override fun onBind(intent: Intent?): IBinder? {
        return if (intent?.action == SERVICE_INTERFACE) {
            super.onBind(intent)
        } else {
            binder
        }
    }

    override fun onCreate() {
        super.onCreate()
        backend = GoBackend(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        if (intent?.getBooleanExtra(EXTRA_DISABLE_KILL_SWITCH, false) == true) {
            killSwitchEnabled = false
            if (_connectionState.value != ConnectionState.CONNECTED) {
                releaseKillSwitchBlock()
                stopForeground(STOP_FOREGROUND_REMOVE)
                stopSelf()
            }
            return START_NOT_STICKY
        }

        if (intent?.getBooleanExtra(EXTRA_ENFORCE_KILL_SWITCH, false) == true) {
            killSwitchEnabled = true
            if (_connectionState.value != ConnectionState.CONNECTED) {
                engageKillSwitchBlock()
                startForegroundWithCurrentState()
            }
        }

        if (intent?.getBooleanExtra(EXTRA_AUTO_CONNECT, false) == true) {
            // Must call startForeground immediately when started as foreground service
            startForegroundWithCurrentState()
            if (_connectionState.value == ConnectionState.DISCONNECTED) {
                handleAutoConnect()
            }
        }
        return START_STICKY
    }

    private fun handleAutoConnect() {
        val prefs = com.escudo.vpn.data.prefs.SecurePrefs(this)
        val serverId = prefs.getSelectedServerId()
        if (serverId == null) {
            android.util.Log.w("EscudoVPN", "Auto-connect: no server selected")
            stopSelf()
            return
        }

        serviceScope.launch {
            try {
                val authInterceptor = com.escudo.vpn.data.api.AuthInterceptor(prefs)
                val client = com.escudo.vpn.data.api.ApiClientFactory.createOkHttpClient(authInterceptor)
                val retrofit = com.escudo.vpn.data.api.ApiClientFactory.createRetrofit(client)
                val apiService = retrofit.create(com.escudo.vpn.data.api.ApiService::class.java)

                val deviceName = "${android.os.Build.MANUFACTURER} ${android.os.Build.MODEL}"
                val response = apiService.connect(
                    com.escudo.vpn.data.model.ConnectRequest(
                        serverId = serverId,
                        deviceName = deviceName,
                        deviceInstallId = prefs.getOrCreateDeviceInstallId(),
                        platform = "android",
                        usageBucket = "normal",
                        preferredClass = "free"
                    )
                )
                prefs.saveDeviceId(response.deviceId)
                val tunnelManager = TunnelManager()
                val config = tunnelManager.parseConfig(response.config)
                val killSwitch = prefs.isKillSwitchEnabled()
                connect(config, killSwitch)
            } catch (e: Exception) {
                if (prefs.isKillSwitchEnabled()) {
                    killSwitchEnabled = true
                    engageKillSwitchBlock()
                    startForegroundWithCurrentState()
                }
                if (BuildConfig.DEBUG) {
                    android.util.Log.e("EscudoVPN", "Auto-connect failed", e)
                } else {
                    android.util.Log.e("EscudoVPN", "Auto-connect failed")
                }
                if (!prefs.isKillSwitchEnabled()) {
                    stopSelf()
                }
            }
        }
    }

    fun connect(config: Config, killSwitch: Boolean) {
        activeConfig = config
        killSwitchEnabled = killSwitch
        manualDisconnectRequested = false
        _connectionState.value = ConnectionState.CONNECTING

        serviceScope.launch {
            try {
                reconnectJob?.cancel()
                if (killSwitchEnabled) {
                    engageKillSwitchBlock()
                }
                if (_connectionState.value == ConnectionState.CONNECTED) {
                    tunnel?.let { existing ->
                        backend?.setState(existing, Tunnel.State.DOWN, null)
                    }
                    stopStatsTracking()
                }
                val tun = EscudoTunnel("escudo")
                tunnel = tun
                startForegroundWithCurrentState()
                releaseKillSwitchBlock()
                backend?.setState(tun, Tunnel.State.UP, config)

                connectionStartTime = System.currentTimeMillis()
                _connectionState.value = ConnectionState.CONNECTED
                startForegroundWithCurrentState()
                startStatsTracking()
            } catch (e: Exception) {
                tunnel = null
                _connectionState.value = ConnectionState.DISCONNECTED
                if (killSwitchEnabled && !manualDisconnectRequested) {
                    engageKillSwitchBlock()
                    startForegroundWithCurrentState()
                }
                if (BuildConfig.DEBUG) {
                    android.util.Log.e("EscudoVPN", "Connect failed", e)
                } else {
                    android.util.Log.e("EscudoVPN", "Connect failed")
                }
                if (killSwitchEnabled && !manualDisconnectRequested) {
                    scheduleReconnect()
                }
            }
        }
    }

    fun disconnect() {
        manualDisconnectRequested = true
        reconnectJob?.cancel()
        _connectionState.value = ConnectionState.DISCONNECTING
        serviceScope.launch {
            try {
                stopStatsTracking()
                tunnel?.let { backend?.setState(it, Tunnel.State.DOWN, null) }
                tunnel = null
                connectionStartTime = 0L
                _trafficStats.value = TrafficStats()
                _connectionTime.value = 0L
                _connectionState.value = ConnectionState.DISCONNECTED
                activeConfig = null
                if (killSwitchEnabled) {
                    engageKillSwitchBlock()
                    startForegroundWithCurrentState()
                } else {
                    releaseKillSwitchBlock()
                    stopForeground(STOP_FOREGROUND_REMOVE)
                    stopSelf()
                }
            } catch (e: Exception) {
                _connectionState.value = ConnectionState.DISCONNECTED
                if (killSwitchEnabled) {
                    engageKillSwitchBlock()
                    startForegroundWithCurrentState()
                } else {
                    releaseKillSwitchBlock()
                }
            }
        }
    }

    private fun startStatsTracking() {
        statsJob = serviceScope.launch {
            while (isActive) {
                delay(1000L)
                if (_connectionState.value == ConnectionState.CONNECTED) {
                    val elapsed = System.currentTimeMillis() - connectionStartTime
                    _connectionTime.value = elapsed / 1000L

                    try {
                        tunnel?.let { tun ->
                            val stats = backend?.getStatistics(tun)
                            if (stats != null) {
                                var totalRx = 0L
                                var totalTx = 0L
                                for (peer in stats.peers()) {
                                    totalRx += stats.peer(peer)?.rxBytes ?: 0L
                                    totalTx += stats.peer(peer)?.txBytes ?: 0L
                                }
                                _trafficStats.value = TrafficStats(rxBytes = totalRx, txBytes = totalTx)
                            }
                        }
                    } catch (_: Exception) {}
                }
            }
        }
    }

    private fun stopStatsTracking() {
        statsJob?.cancel()
        statsJob = null
    }

    private fun scheduleReconnect() {
        val config = activeConfig ?: return
        if (reconnectJob?.isActive == true) return

        reconnectJob = serviceScope.launch {
            _connectionState.value = ConnectionState.CONNECTING
            startForegroundWithCurrentState()
            delay(1500L)
            if (!manualDisconnectRequested) {
                connect(config, true)
            }
        }
    }

    private fun engageKillSwitchBlock() {
        if (killSwitchInterface != null) return

        try {
            val builder = Builder()
                .setSession("Escudo Kill Switch")
                .setBlocking(false)
                .addAddress("10.255.255.1", 32)
                .addRoute("0.0.0.0", 0)

            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.LOLLIPOP) {
                builder.addAddress("fd00:1:fd00:1::1", 128)
                builder.addRoute("::", 0)
            }

            killSwitchInterface = builder.establish()
        } catch (e: Exception) {
            if (BuildConfig.DEBUG) {
                android.util.Log.e("EscudoVPN", "Failed to engage kill switch block", e)
            } else {
                android.util.Log.e("EscudoVPN", "Failed to engage kill switch block")
            }
        }
    }

    private fun releaseKillSwitchBlock() {
        try {
            killSwitchInterface?.close()
        } catch (_: Exception) {
        } finally {
            killSwitchInterface = null
        }
    }

    private fun startForegroundWithCurrentState() {
        if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.UPSIDE_DOWN_CAKE) {
            startForeground(
                NOTIFICATION_ID,
                createNotification(),
                android.content.pm.ServiceInfo.FOREGROUND_SERVICE_TYPE_SPECIAL_USE
            )
        } else {
            startForeground(NOTIFICATION_ID, createNotification())
        }
    }

    private fun createNotification(): Notification {
        val intent = Intent(this, MainActivity::class.java)
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE
        )

        val contentText = when {
            _connectionState.value == ConnectionState.CONNECTED ->
                getString(R.string.vpn_notification_connected)
            killSwitchEnabled && killSwitchInterface != null ->
                "Internet bloqueada ate a VPN reconectar"
            _connectionState.value == ConnectionState.CONNECTING ->
                "Conectando VPN..."
            else ->
                "VPN desconectada"
        }

        return NotificationCompat.Builder(this, EscudoApp.VPN_CHANNEL_ID)
            .setContentTitle(getString(R.string.vpn_notification_title))
            .setContentText(contentText)
            .setSmallIcon(android.R.drawable.ic_lock_lock)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .build()
    }

    override fun onDestroy() {
        serviceScope.cancel()
        try {
            tunnel?.let { backend?.setState(it, Tunnel.State.DOWN, null) }
        } catch (_: Exception) {}
        releaseKillSwitchBlock()
        super.onDestroy()
    }

    companion object {
        const val NOTIFICATION_ID = 1
        const val EXTRA_AUTO_CONNECT = "extra_auto_connect"
        const val EXTRA_ENFORCE_KILL_SWITCH = "extra_enforce_kill_switch"
        const val EXTRA_DISABLE_KILL_SWITCH = "extra_disable_kill_switch"
    }

    private inner class EscudoTunnel(private val tunnelName: String) : Tunnel {
        override fun getName(): String = tunnelName
        override fun onStateChange(newState: Tunnel.State) {
            if (newState == Tunnel.State.DOWN && !manualDisconnectRequested) {
                tunnel = null
                stopStatsTracking()
                _trafficStats.value = TrafficStats()
                _connectionTime.value = 0L
                if (killSwitchEnabled) {
                    _connectionState.value = ConnectionState.DISCONNECTED
                    engageKillSwitchBlock()
                    startForegroundWithCurrentState()
                    scheduleReconnect()
                } else {
                    _connectionState.value = ConnectionState.DISCONNECTED
                }
            }
        }
    }
}
